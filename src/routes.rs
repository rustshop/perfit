pub mod account;
mod auth;
pub mod error;
pub mod metric;
pub mod token;

use std::ops;

use axum::body::Body;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use reqwest::header::ACCEPT_ENCODING;
use time::OffsetDateTime;

use self::account::account_new;
use self::error::{RequestError, RequestResult, UserErrorResponse, UserRequestError};
use self::metric::{
    get_metric, metric_find, metric_get, metric_get_default_type, metric_new, metric_post,
    MetricOpts,
};
use self::token::token_new;
use crate::db::DataPointRecord;
use crate::fragment::{self};
use crate::models::ts::{DateTimeExt, Ts};
use crate::models::MetricId;
use crate::state::SharedAppState;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(RequestError))]
struct AppJson<T>(T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

const MAX_DATA_POINTS_LIMIT: usize = 1000;

pub async fn render_svg(
    state: &SharedAppState,
    metric_id: MetricId,
    opts: &MetricOpts,
) -> color_eyre::Result<(String, ops::Range<OffsetDateTime>)> {
    Ok(render_svg_from_measurements(
        &get_metric(state, metric_id, opts).await?,
        opts,
    ))
}

fn render_svg_from_measurements(
    measurements: &[(Ts, DataPointRecord)],
    opts: &MetricOpts,
) -> (String, ops::Range<OffsetDateTime>) {
    use poloto::build;

    fn nan_out_of_range(val: f64, min_opt: Option<f64>, max_opt: Option<f64>) -> f64 {
        if max_opt.is_some_and(|max_val| max_val < val)
            || min_opt.is_some_and(|min_val| val < min_val)
        {
            f64::NAN
        } else {
            val
        }
    }
    fn saturate_out_of_range(mut val: f64, min_opt: Option<f64>, max_opt: Option<f64>) -> f64 {
        if let Some(max_val) = max_opt {
            val = max_val.min(val);
        }
        if let Some(min_val) = min_opt {
            val = min_val.max(val);
        }
        val
    }

    let start_rel_ts = measurements.first().map(|m| m.0).unwrap_or_default();
    let start_bound_datetime = start_rel_ts.to_datetime().round_down_to_hour();
    let end_rel_ts = measurements.last().map(|m| m.0).unwrap_or_default();
    let end_bound_datetime = end_rel_ts.to_datetime().round_up_exclusive_to_hour();

    let range = end_bound_datetime - start_bound_datetime;
    let hours_as_secs = 60. * 60.;

    let tick_step_secs = (range.as_seconds_f64() / 1.5 / hours_as_secs).ceil() * hours_as_secs;

    let datapoints = measurements.iter().map(|(ts, m)| {
        let y = m.value.as_f32() as f64;
        let x = ts.to_absolute_secs() as f64;
        (x, y)
    });

    let xticks = poloto::ticks::TickDistribution::new(std::iter::successors(
        Some(start_bound_datetime.unix_timestamp() as f64),
        |w| Some(w + tick_step_secs),
    ))
    .with_tick_fmt(|&v| {
        OffsetDateTime::from_unix_timestamp(v as i64)
            .expect("Can't fail")
            .our_fmt()
            .to_string()
    });

    let frame = poloto::frame_build()
        .data(poloto::plots!(
            build::plot("").scatter(
                datapoints
                    .clone()
                    .map(|(x, y)| [x, saturate_out_of_range(y, opts.min, opts.max)])
            ),
            build::plot("")
                .line(datapoints.map(|(x, y)| [x, nan_out_of_range(y, opts.min, opts.max)])),
            poloto::build::markers(
                [
                    start_bound_datetime.unix_timestamp() as f64,
                    end_bound_datetime.unix_timestamp() as f64
                ],
                [opts.min, opts.max].into_iter().flatten()
            )
        ))
        .map_xticks(|_| xticks)
        .build_and_label((
            opts.title.clone(),
            opts.x_label.clone(),
            opts.y_label.clone(),
        ));

    (
        frame
            .append_to(poloto::header().light_theme())
            .render_string()
            .expect("Can't fail?"),
        start_bound_datetime..end_bound_datetime,
    )
}

pub fn static_file_handler(state: SharedAppState) -> Router {
    Router::new()
        .route(
            "/:file",
            get(
                |state: State<SharedAppState>, path: Path<String>, req_headers: HeaderMap| async move {
                    let Some(asset) = state.assets.get_from_path(&path) else {
                        return StatusCode::NOT_FOUND.into_response();
                    };

                    let mut resp_headers = HeaderMap::new();

                    // We set the content type explicitly here as it will otherwise
                    // be inferred as an `octet-stream`
                    resp_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static(
                            asset.content_type().unwrap_or("application/octet-stream"),
                        ),
                    );

                    let accepts_brotli = req_headers.get_all(ACCEPT_ENCODING)
                        .into_iter().any(|encodings| {
                            let Ok(str) = encodings.to_str() else { return false };

                            str.split(',').any(|s| s.trim() == "br")

                          });

                    let content =
                        if accepts_brotli {
                            if let Some(compressed) = asset.compressed.as_ref() {
                                resp_headers.insert(CONTENT_ENCODING, HeaderValue::from_static("br"));

                                compressed.clone()
                            } else {
                            asset.raw.clone()
                        }
                    } else {
                        asset.raw.clone()
                    };

                    (resp_headers, content).into_response()
                },
            ),
        )
        .layer(middleware::from_fn(cache_control))
        .with_state(state)
}

pub async fn cache_control(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
        const CACHEABLE_CONTENT_TYPES: &[(&str, u32)] = &[
            ("text/html", 60),
            ("image/svg+xml", 60),
            ("text/css", 60 * 60 * 24),
            ("application/javascript", 60 * 60 * 24),
        ];

        if let Some(&(_, secs)) = CACHEABLE_CONTENT_TYPES
            .iter()
            .find(|&(ct, _)| content_type.as_bytes().starts_with(ct.as_bytes()))
        {
            let value = format!("public, max-age={}", secs);

            if let Ok(value) = HeaderValue::from_str(&value) {
                response.headers_mut().insert("cache-control", value);
            }
        }
    }

    response
}

pub async fn index() -> RequestResult<impl IntoResponse> {
    Ok(Html(fragment::index()?.into_string()))
}

pub async fn not_found(_state: State<SharedAppState>, _req: Request<Body>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        AppJson(UserErrorResponse {
            message: "Not Found".to_string(),
        }),
    )
}

pub fn route_handler(state: SharedAppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/a/", put(account_new))
        .route("/t/", put(token_new))
        .route("/m/", put(metric_new).get(metric_find))
        .route("/m/:metric", post(metric_post).get(metric_get_default_type))
        .route("/m/:metric/:type", get(metric_get))
        .fallback(not_found)
        .with_state(state)
        .layer(middleware::from_fn(cache_control))
}
