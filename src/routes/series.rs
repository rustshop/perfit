use std::ops;

use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::Json;
use resiter::AndThen as _;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

use super::auth::Auth;
use super::{render_svg, RequestResult, UserRequestError, MAX_DATA_POINTS_LIMIT};
use crate::db::{
    Sample, SampleRecord, SampleValue, SeriesRecord, TABLE_SAMPLES, TABLE_SERIES, TABLE_SERIES_REV,
};
use crate::fragment::render_chart_form;
use crate::models::ts::Ts;
use crate::models::{SeriesId, SeriesInternalId};
use crate::serde::deserialize_opt_f64_from_empty_string;
use crate::state::SharedAppState;

#[instrument]
pub async fn series_new(
    State(state): State<SharedAppState>,
    Auth(auth): Auth,
) -> RequestResult<Json<SeriesId>> {
    let series = state
        .db
        .write_with(|tx| {
            let mut table_series_rev = tx.open_table(&TABLE_SERIES_REV)?;
            let new_internal_id = table_series_rev
                .last()?
                .map(|(k, _v)| k.value().next())
                .unwrap_or_default();

            let series = SeriesId::generate();

            tx.open_table(&TABLE_SERIES)?.insert(
                &series,
                &SeriesRecord {
                    created: Ts::now(),
                    account_id: auth.account_id,
                    internal_id: new_internal_id,
                },
            )?;
            table_series_rev.insert(&new_internal_id, &series)?;

            Ok(series)
        })
        .await?;

    Ok(Json(series))
}

#[instrument]
pub async fn series_post(
    State(state): State<SharedAppState>,
    Path(series): Path<SeriesId>,
    Json(payload): Json<SampleValue>,
) -> RequestResult<Json<u64>> {
    let ts = state
        .db
        .write_with(|tx| {
            let series_record = tx
                .open_table(&TABLE_SERIES)?
                .get(&series)?
                .ok_or(UserRequestError::SeriesNotFound(series))?
                .value();

            let ts = Ts::now();

            tx.open_table(&TABLE_SAMPLES)?.insert(
                &Sample {
                    series_internal_id: series_record.internal_id,
                    ts,
                },
                &SampleRecord { value: payload },
            )?;

            Ok(ts)
        })
        .await?;

    Ok(Json(ts.to_absolute_secs()))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SeriesOpts {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub x_label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub y_label: String,
    #[serde(with = "crate::serde::custom_rfc3339_option", default)]
    pub start: Option<OffsetDateTime>,
    #[serde(with = "crate::serde::custom_rfc3339_option", default)]
    pub end: Option<OffsetDateTime>,
    #[serde(deserialize_with = "deserialize_opt_f64_from_empty_string", default)]
    pub min: Option<f64>,
    #[serde(deserialize_with = "deserialize_opt_f64_from_empty_string", default)]
    pub max: Option<f64>,
}

impl SeriesOpts {
    pub fn key_range(&self, metric_internal_id: SeriesInternalId) -> ops::Range<Sample> {
        Sample {
            series_internal_id: metric_internal_id,
            ts: self.start.map(Ts::from).unwrap_or(Ts::ZERO),
        }
            ..self
                .end
                .map(|t| Sample {
                    series_internal_id: metric_internal_id,
                    ts: Ts::from(t),
                })
                .unwrap_or(Sample {
                    series_internal_id: metric_internal_id.next(),
                    ts: Ts::ZERO,
                })
    }
}

pub async fn get_samples(
    state: &SharedAppState,
    series_id: SeriesId,
    opts: &SeriesOpts,
) -> color_eyre::Result<Vec<(Ts, SampleRecord)>> {
    state
        .db
        .read_with(|tx| {
            let series_record = tx
                .open_table(&TABLE_SERIES)?
                .get(&series_id)?
                .ok_or(UserRequestError::SeriesNotFound(series_id))?
                .value();

            let samples: Vec<_> = tx
                .open_table(&TABLE_SAMPLES)?
                .range(opts.key_range(series_record.internal_id))?
                .and_then_ok(|(k, v)| Ok((k.value().ts, v.value())))
                .take(MAX_DATA_POINTS_LIMIT)
                .collect::<Result<_, _>>()?;

            Ok(samples)
        })
        .await
}

#[instrument]
pub async fn series_get_default_type(
    State(state): State<SharedAppState>,
    Path(series): Path<SeriesId>,
    Query(opts): Query<SeriesOpts>,
    uri: Uri,
) -> RequestResult<impl IntoResponse> {
    Ok(render_chart_form(&state, series, &opts)
        .await?
        .into_response())
}

#[instrument]
pub async fn series_get(
    State(state): State<SharedAppState>,
    Path((series_id, r#type)): Path<(SeriesId, String)>,
    Query(opts): Query<SeriesOpts>,
    uri: Uri,
) -> RequestResult<impl IntoResponse> {
    Ok(match r#type.as_str() {
        "html" | "" => render_chart_form(&state, series_id, &opts)
            .await?
            .into_response(),
        "svg" => {
            let (svg, _time_bound) = render_svg(&state, series_id, &opts).await?;

            ([(CONTENT_TYPE, "image/svg+xml")], svg).into_response()
        }
        "json" => {
            let samples = get_samples(&state, series_id, &opts).await?;

            Json(samples).into_response()
        }
        _ => {
            return Err(UserRequestError::FormatNotSupported.into());
        }
    })
}
