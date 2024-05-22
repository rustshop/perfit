use std::ops::{self, Sub};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::Uri;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use reqwest::StatusCode;
use resiter::AndThen as _;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

use super::auth::Auth;
use super::{render_svg, RequestResult, UserRequestError, MAX_DATA_POINTS_LIMIT};
use crate::db::{
    DataPoint, DataPointMetadata, DataPointRecord, DataPointValue, MetricRecord, TABLE_DATA_POINTS,
    TABLE_METRICS, TABLE_METRICS_REV,
};
use crate::fragment::render_chart_form;
use crate::models::ts::Ts;
use crate::models::{MetricId, MetricInternalId};
use crate::state::SharedAppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct MetricGetPayload {
    metric_id: MetricId,
}

#[instrument]
pub async fn metric_find(
    State(state): State<SharedAppState>,
    Query(MetricGetPayload { metric_id }): Query<MetricGetPayload>,
) -> RequestResult<Response> {
    if let Some(_metric_record) = state
        .db
        .read_with(|tx| {
            Ok(tx
                .open_table(&TABLE_METRICS)?
                .get(&metric_id)?
                .map(|record| record.value()))
        })
        .await?
    {
        Ok(([("HX-Location", format!("/m/{}", metric_id))], Html("")).into_response())
    } else {
        Ok((StatusCode::NOT_FOUND, Html("Metric not found")).into_response())
    }
}

#[instrument]
pub async fn metric_new(
    State(state): State<SharedAppState>,
    Auth(auth): Auth,
) -> RequestResult<Json<MetricId>> {
    let metric_id = state
        .db
        .write_with(|tx| {
            let mut table_metric_rev = tx.open_table(&TABLE_METRICS_REV)?;
            let new_internal_id = table_metric_rev
                .last()?
                .map(|(k, _v)| k.value().next())
                .unwrap_or_default();

            let metric_id = MetricId::generate();

            tx.open_table(&TABLE_METRICS)?.insert(
                &metric_id,
                &MetricRecord {
                    created: Ts::now(),
                    account_id: auth.account_id,
                    internal_id: new_internal_id,
                },
            )?;
            table_metric_rev.insert(&new_internal_id, &metric_id)?;

            Ok(metric_id)
        })
        .await?;

    Ok(Json(metric_id))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct MetricPostPayload {
    value: DataPointValue,
    metadata: Option<DataPointMetadata>,
}

#[instrument]
pub async fn metric_post(
    State(state): State<SharedAppState>,
    Path(metric_id): Path<MetricId>,
    Json(MetricPostPayload { value, metadata }): Json<MetricPostPayload>,
) -> RequestResult<Json<u64>> {
    let ts = state
        .db
        .write_with(|tx| {
            let metric_record = tx
                .open_table(&TABLE_METRICS)?
                .get(&metric_id)?
                .ok_or(UserRequestError::MetricNotFound(metric_id))?
                .value();

            let ts = Ts::now();

            let mut data_points_table = tx.open_table(&TABLE_DATA_POINTS)?;

            let idx = data_points_table
                .range(
                    &DataPoint {
                        metric_internal_id: metric_record.internal_id,
                        ts,
                        idx: 0,
                    }..&DataPoint {
                        metric_internal_id: metric_record.internal_id,
                        ts: ts.inc(),
                        idx: 0,
                    },
                )?
                .next_back()
                .transpose()?
                .map(|(k, _v)| k.value().idx + 1)
                .unwrap_or_default();

            data_points_table.insert(
                &DataPoint {
                    metric_internal_id: metric_record.internal_id,
                    ts,
                    idx,
                },
                &DataPointRecord {
                    value,
                    metadata: metadata.unwrap_or_default(),
                },
            )?;

            Ok(ts)
        })
        .await?;

    Ok(Json(ts.to_absolute_secs()))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct MetricOpts {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub x_label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub y_label: String,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub start_fixed: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub end_fixed: Option<OffsetDateTime>,
    #[serde(with = "humantime_serde::option", default)]
    pub start_rel: Option<Duration>,
    #[serde(with = "humantime_serde::option", default)]
    pub end_rel: Option<Duration>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

impl MetricOpts {
    pub fn key_range(&self, metric_internal_id: MetricInternalId) -> ops::Range<DataPoint> {
        let now = std::time::SystemTime::now();
        DataPoint {
            metric_internal_id,
            ts: self.start(now).unwrap_or(Ts::ZERO),
            idx: 0,
        }
            ..self
                .end(now)
                .map(|t| DataPoint {
                    metric_internal_id,
                    ts: t,
                    idx: 0,
                })
                .unwrap_or(DataPoint {
                    metric_internal_id: metric_internal_id.next(),
                    ts: Ts::ZERO,
                    idx: 0,
                })
    }

    fn end(&self, now: std::time::SystemTime) -> Option<Ts> {
        self.end_rel
            .map(|duration| Ts::from(now.sub(duration)))
            .or_else(|| self.end_fixed.map(Ts::from))
    }

    fn start(&self, now: std::time::SystemTime) -> Option<Ts> {
        self.start_rel
            .map(|duration| Ts::from(now.sub(duration)))
            .or_else(|| self.start_fixed.map(Ts::from))
    }
}

pub async fn get_metric(
    state: &SharedAppState,
    metric_id: MetricId,
    opts: &MetricOpts,
) -> color_eyre::Result<Vec<(Ts, DataPointRecord)>> {
    state
        .db
        .read_with(|tx| {
            let metric_record = tx
                .open_table(&TABLE_METRICS)?
                .get(&metric_id)?
                .ok_or(UserRequestError::MetricNotFound(metric_id))?
                .value();

            let data_points: Vec<_> = tx
                .open_table(&TABLE_DATA_POINTS)?
                .range(opts.key_range(metric_record.internal_id))?
                .enumerate()
                // We don't want ever to stop at the boundary of multiple data points for the same
                // second so instead of a simple `.take(limit), we need something
                // more complex
                .take_while(|(i, d)| {
                    i < &MAX_DATA_POINTS_LIMIT
                        || d.as_ref()
                            .map(|(k, _v)| k.value().idx != 0)
                            .unwrap_or_default()
                })
                .map(|(_, r)| r)
                .and_then_ok(|(k, v)| Ok((k.value().ts, v.value())))
                .collect::<Result<_, _>>()?;

            Ok(data_points)
        })
        .await
}

#[instrument]
pub async fn metric_get_default_type(
    State(state): State<SharedAppState>,
    Path(metric_id): Path<MetricId>,
    Query(opts): Query<MetricOpts>,
    uri: Uri,
) -> RequestResult<impl IntoResponse> {
    Ok(render_chart_form(&state, metric_id, &opts)
        .await?
        .into_response())
}

#[derive(Debug, Clone, Serialize)]
pub struct RawMetricGetBodyRecord {
    t: Ts,
    v: DataPointValue,
    #[serde(skip_serializing_if = "DataPointMetadata::is_empty")]
    m: DataPointMetadata,
}

#[instrument]
pub async fn metric_get(
    State(state): State<SharedAppState>,
    Path((metric_id, r#type)): Path<(MetricId, String)>,
    Query(opts): Query<MetricOpts>,
    uri: Uri,
) -> RequestResult<impl IntoResponse> {
    Ok(match r#type.as_str() {
        "html" | "" => render_chart_form(&state, metric_id, &opts)
            .await?
            .into_response(),
        "svg" => {
            let (svg, _time_bound) = render_svg(&state, metric_id, &opts).await?;

            ([(CONTENT_TYPE, "image/svg+xml")], svg).into_response()
        }
        "json" => {
            let data_points: Vec<RawMetricGetBodyRecord> = get_metric(&state, metric_id, &opts)
                .await?
                .into_iter()
                .map(
                    |(ts, DataPointRecord { value, metadata })| RawMetricGetBodyRecord {
                        t: ts,
                        v: value,
                        m: metadata,
                    },
                )
                .collect();

            Json(data_points).into_response()
        }
        _ => {
            return Err(UserRequestError::FormatNotSupported.into());
        }
    })
}
