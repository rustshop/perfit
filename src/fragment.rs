use axum::response::{Html, IntoResponse};
use maud::{html, Markup, DOCTYPE};

use crate::models::MetricId;
use crate::routes::error::{RequestResult, UserRequestError};
use crate::routes::metric::MetricOpts;
use crate::routes::render_svg;
use crate::state::SharedAppState;

const LABEL_CLASS: &str = "block mb-2 text-sm font-medium text-gray-900 dark:text-white";
const TEXT_INPUT_CLASS: &str = "shadow-sm bg-gray-50 border border-gray-300 text-gray-900 sm:text-sm rounded-lg focus:ring-primary-500 focus:border-primary-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-primary-500 dark:focus:border-primary-500";

pub fn page(title: &str, content: Markup) -> Markup {
    /// A basic header with a dynamic `page_title`.
    pub(crate) fn head(page_title: &str) -> Markup {
        html! {
            (DOCTYPE)
            html lang="en";
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" type="text/css" href="/assets/style.css";
                link rel="stylesheet" type="text/css" href="/assets/style-htmx-send-error.css";
                title { (page_title) }
            }
        }
    }

    pub(crate) fn header() -> Markup {
        html! {
            header ."container py-5 flex flex-row place-content-center gap-6 items-center" {
                    div  { "PerfIt" }
            }
        }
    }

    /// A static footer.
    pub(crate) fn footer() -> Markup {
        html! {
            script src="https://unpkg.com/htmx.org@1.9.12" {};
            script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/response-targets.js" {};
            script type="module" src="/assets/script.js" {};
            script type="module" src="/assets/script-htmx-send-error.js" {};
        }
    }

    html! {
        (head(title))
        body ."container relative mx-auto !block" style="display: none" {
            div #"gray-out-page" ."fixed inset-0 send-error-hidden"  {
                div ."relative z-50 bg-white mx-auto max-w-sm p-10 flex flex-center flex-col gap-2" {
                    p { "Connection error" }
                    button ."rounded bg-red-700 text-white px-2 py-1" hx-get="/" hx-target="body" hx-swap="outerHTML" { "Reload" }
                }
                div ."inset-0 absolute z-0 bg-gray-500 opacity-50" {}
            }
            (header())

            main ."container flex flex-col items-center justify-center" {
                (content)
            }
            (footer())
        }
    }
}

pub async fn render_chart_form(
    state: &SharedAppState,
    metric_id: MetricId,
    opts: &MetricOpts,
) -> RequestResult<impl IntoResponse> {
    let (svg, time_bound) = render_svg(state, metric_id, opts).await?;
    let params = serde_qs::to_string(&opts).map_err(|_| UserRequestError::InvalidPath)?;

    const TIME_FORMAT: &[time::format_description::FormatItem<'static>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

    let (input_start_rel_value, input_start_fixed_value) = if let Some(start_rel) = opts.start_rel {
        (
            humantime_serde::re::humantime::format_duration(start_rel).to_string(),
            "".into(),
        )
    } else {
        (
            "".into(),
            opts.start_fixed
                .and_then(|f| f.format(&TIME_FORMAT).ok())
                .unwrap_or_else(|| time_bound.start.format(&TIME_FORMAT).expect("Valid format")),
        )
    };
    let (input_end_rel_value, input_end_fixed_value) = if let Some(end_rel) = opts.end_rel {
        (
            humantime_serde::re::humantime::format_duration(end_rel).to_string(),
            "".into(),
        )
    } else {
        (
            "".into(),
            opts.end_fixed
                .and_then(|f| f.format(&TIME_FORMAT).ok())
                .unwrap_or_else(|| time_bound.end.format(&TIME_FORMAT).expect("Valid format")),
        )
    };

    let page_title = if opts.title.is_empty() {
        metric_id.to_string()
    } else {
        opts.title.clone()
    };
    Ok(Html(
        page(
            &format!("Chart: {}", page_title),
            maud::html! {
                form
                    hx-get=(state.html_chart_url(metric_id))
                    hx-push-url="true"
                    hx-trigger="change from:(form input) delay:0.5s, keyup delay:0.5s"
                    hx-target="find #svg-img"
                    hx-swap="outerHTML"
                    hx-select="#svg-img"
                    hx-sync="this:replace"
                    id="metric-chart-form"
                {
                    div class="grid grid-cols-6 gap-6" {

                        div class="col-span-6 relative" id="svg-img" {
                            (maud::PreEscaped(svg))
                            a
                                class="absolute bottom-4 right-6 hover:text-blue-600"
                                href=(format!("{}?{}", state.svg_chart_url(metric_id), params)) {
                                "Export..."
                            }
                        }

                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="min"
                                class=(LABEL_CLASS)
                                { "Min" }

                            input
                                id="min"
                                class=(TEXT_INPUT_CLASS)
                                name="min"
                                type="number"
                                value=(opts.min.map(|f| f.to_string()).unwrap_or_else(|| "".into()));
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="max"
                                class=(LABEL_CLASS)
                                { "Max" }
                            input id="max"
                                class=(TEXT_INPUT_CLASS)
                                name="max"
                                type="number"
                                value=(opts.max.map(|f| f.to_string()).unwrap_or_else(|| "".into()));
                        }

                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="title"
                                class=(LABEL_CLASS)
                                { "Title" }
                            input
                                class=(TEXT_INPUT_CLASS)
                                id="title"
                                name="title"
                                type="text"
                                placeholder="Title..."
                                value=(opts.title);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="y-label"
                                class=(LABEL_CLASS)
                                { "Y Label" }
                            input
                                class=(TEXT_INPUT_CLASS)
                                id="y-label"
                                name="y-label"
                                type="text"
                                placeholder="Y Label..."
                                value=(opts.y_label);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="start-rel"
                                class=(LABEL_CLASS)
                                { "Start (rel)" }

                            input
                                id="start-rel"
                                class=(TEXT_INPUT_CLASS)
                                name="start-rel"
                                type="text"
                                placeholder="2 weeks ..."
                                value=(input_start_rel_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="end-rel"
                                class=(LABEL_CLASS)
                                { "End (rel)" }

                            input
                                id="end-rel"
                                class=(TEXT_INPUT_CLASS)
                                name="end-rel"
                                type="text"
                                placeholder="0s ..."
                                value=(input_end_rel_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="start-fixed"
                                class=(LABEL_CLASS)
                                { "Start (fixed)" }

                            input
                                id="start-fixed"
                                class=(TEXT_INPUT_CLASS)
                                name="start-fixed"
                                type="text"
                                value=(input_start_fixed_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="end-fixed"
                                class=(LABEL_CLASS)
                                { "End (fixed)" }

                            input
                                id="end-fixed"
                                class=(TEXT_INPUT_CLASS)
                                name="end-fixed"
                                type="text"
                                value=(input_end_fixed_value);
                        }
                    }
                }
            },
        )
        .into_string(),
    )
    .into_response())
}

pub fn index() -> color_eyre::Result<Markup> {
    let content = html! {
        div ."max-w-md mx-auto bg-white p-6 rounded-lg shadow-md my-6" {
            p ."p-2" {
                "PerfIt is a tiny web service that tracks and plots metrics: typically time it takes to execute things in CI-pipelines. "

                "Read more at "
                a href="https://github.com/rustshop/perfit" class="text-blue-500 hover:text-blue-800" { "PerfIt github page" } "."
            }
            p ."p-2" {
                "For most operations you want to use " span ."inline-block font-mono bg-gray-200 text-gray-800 px-1 rounded" { "perfit" } "command line client, but you can view metrics and customize charts for them interactively using the form below."
            }
        }
        div ."bg-white p-6 rounded-lg shadow-md my-6" hx-ext="response-targets" {

                form
                    hx-get="/m/"
                    hx-target="this"
                    hx-target-error="#error"
                    hx-swap="innerHTML"
                    hx-sync="this:replace"
                    id="metric-find"
                {
                    div id="error" class="flex flex-row p-1"  { }

                    div class="flex flex-row" {

                        div class="px-2" {
                            input
                                id="metric-id"
                                class=(TEXT_INPUT_CLASS)
                                name="metric-id"
                                type="text"
                                placeholder = "Metric ID ..."
                                value="";
                        }

                        div class="px-2" {
                            button
                                type="submit"
                                ."bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" {
                                    "Find"
                                }
                        }
                    }
                }
        }



    };

    Ok(page("PerfIt!", content))
}
