use axum::response::{Html, IntoResponse};
use maud::{html, Markup, DOCTYPE};

use crate::models::MetricId;
use crate::routes::error::{RequestResult, UserRequestError};
use crate::routes::metric::MetricOpts;
use crate::routes::render_svg;
use crate::state::SharedAppState;

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
            script src="https://unpkg.com/htmx.org@1.9.11" {};
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

            main ."container flex items-center justify-center" {
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

    let input_class="shadow-sm bg-gray-50 border border-gray-300 text-gray-900 sm:text-sm rounded-lg focus:ring-primary-500 focus:border-primary-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-primary-500 dark:focus:border-primary-500";
    let label_class = "block mb-2 text-sm font-medium text-gray-900 dark:text-white";
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
                                class=(label_class)
                                { "Min" }

                            input
                                id="min"
                                class=(input_class)
                                name="min"
                                type="number"
                                value=(opts.min.map(|f| f.to_string()).unwrap_or_else(|| "".into()));
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="max"
                                class=(label_class)
                                { "Max" }
                            input id="max"
                                class=(input_class)
                                name="max"
                                type="number"
                                value=(opts.max.map(|f| f.to_string()).unwrap_or_else(|| "".into()));
                        }

                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="title"
                                class=(label_class)
                                { "Title" }
                            input
                                class=(input_class)
                                id="title"
                                name="title"
                                type="text"
                                placeholder="Title..."
                                value=(opts.title);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="y-label"
                                class=(label_class)
                                { "Y Label" }
                            input
                                class=(input_class)
                                id="y-label"
                                name="y-label"
                                type="text"
                                placeholder="Y Label..."
                                value=(opts.y_label);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="start-rel"
                                class=(label_class)
                                { "Start (rel)" }

                            input
                                id="start-rel"
                                class=(input_class)
                                name="start-rel"
                                type="text"
                                placeholder="2 weeks ..."
                                value=(input_start_rel_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="end-rel"
                                class=(label_class)
                                { "End (rel)" }

                            input
                                id="end-rel"
                                class=(input_class)
                                name="end-rel"
                                type="text"
                                placeholder="0s ..."
                                value=(input_end_rel_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="start-fixed"
                                class=(label_class)
                                { "Start (fixed)" }

                            input
                                id="start-fixed"
                                class=(input_class)
                                name="start-fixed"
                                type="text"
                                value=(input_start_fixed_value);
                        }
                        div class="col-span-6 sm:col-span-3" {
                            label
                                for="end-fixed"
                                class=(label_class)
                                { "End (fixed)" }

                            input
                                id="end-fixed"
                                class=(input_class)
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
    // use jotdown::Render;
    // let djot_input = include_str!("../README.md");
    // let events = jotdown::Parser::new(djot_input);
    // let mut html = String::new();
    // jotdown::html::Renderer::default()
    //     .push(events, &mut html)
    //     .map_err(|_e| UserRequestError::AssertionError)?;

    let content = html! {

        p {
            "PerfIt is a tiny web service that tracks and plots metrics: typically time it takes to execute things in CI-pipelines. "

            "Read more at "
            a href="https://github.com/rustshop/perfit"
            class="hover:text-blue-600"
            { "PerfIt github page" } "."
        }

    };

    Ok(page("PerfIt!", content))
}
