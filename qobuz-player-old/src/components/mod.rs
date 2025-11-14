use leptos::{IntoView, component, prelude::*};
use qobuz_player_controls::notification;
use serde::Deserialize;

use crate::{html, icons::Star};
pub(crate) mod list;

#[derive(Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tab {
    Albums,
    Artists,
    Playlists,
}

pub(crate) struct Duration {
    pub minutes: u32,
}

pub(crate) fn parse_duration(seconds: u32) -> Duration {
    Duration {
        minutes: seconds / 60,
    }
}

pub(crate) fn button_class() -> String {
    "flex gap-2 justify-center items-center py-2 px-4 w-full bg-blue-500 rounded cursor-pointer active:bg-blue-700 whitespace-nowrap"
        .into()
}

#[component]
pub(crate) fn description(description: Option<String>, entity_title: String) -> impl IntoView {
    description.map(|description| {
        html! {
            <div class="flex flex-col gap-4 p-4 bg-gray-800 inset-shadow-lg">
                <h3 class="text-lg">{format!("About {entity_title}")}</h3>
                <p>{description}</p>
            </div>
        }
    })
}

#[component]
pub(crate) fn button_group(children: ChildrenFragment) -> impl IntoView {
    let nodes = children()
        .nodes
        .into_iter()
        .filter(|n| n.html_len() > 10)
        .collect::<Vec<_>>();

    let even = nodes.len() % 2 == 0;

    html! {
        <div class=format!(
            "grid grid-cols-2 {} gap-4",
            if !even { "*:last:col-span-2" } else { "" },
        )>{nodes}</div>
    }
}

pub(crate) fn toast(message: notification::Notification) -> impl IntoView {
    let (message, severity) = match message {
        notification::Notification::Error(message) => (message, 1),
        notification::Notification::Warning(message) => (message, 2),
        notification::Notification::Success(message) => (message, 3),
        notification::Notification::Info(message) => (message, 4),
    };
    html! {
        <div
            class=format!(
                "block p-4 shadow max-w-sm rounded-lg text-wrap text-white {}",
                if severity == 1 {
                    "bg-red-500"
                } else if severity == 2 {
                    "bg-yellow-500"
                } else if severity == 3 {
                    "bg-teal-500"
                } else {
                    "bg-blue-500"
                },
            )
            remove-me="3s"
        >
            {message}
        </div>
    }
}

#[component]
pub(crate) fn toggle_favorite(id: String, is_favorite: bool) -> impl IntoView {
    html! {
        <button
            class=button_class()
            id="toggle-favorite"
            hx-swap="outerHTML"
            hx-target="this"
            hx-put=format!("{}/{}", id, if is_favorite { "unset-favorite" } else { "set-favorite" })
        >
            <span class="size-6">
                <Star solid=is_favorite />
            </span>
            <span>Favorite</span>
        </button>
    }
}

#[component]
pub(crate) fn info(hires_available: bool, explicit: bool) -> impl IntoView {
    html! {
        {(explicit || hires_available)
            .then(|| {
                html! {
                    <div class="flex gap-2 items-center">
                        {explicit
                            .then(|| {
                                html! {
                                    <div class="text-gray-400 whitespace-nowrap size-6">
                                        <svg
                                            height="24"
                                            viewBox="0 0 24 24"
                                            xmlns="http://www.w3.org/2000/svg"
                                        >
                                            <path
                                                d="M21 3H3v18h18V3zm-6 6h-4v2h4v2h-4v2h4v2H9V7h6v2z"
                                                fill="currentColor"
                                            />
                                        </svg>
                                    </div>
                                }
                            })}
                        {hires_available
                            .then(|| {
                                html! {
                                    <div class="text-gray-400 whitespace-nowrap size-6">
                                        <svg
                                            height="24"
                                            viewBox="0 0 256 256"
                                            xmlns="http://www.w3.org/2000/svg"
                                        >
                                            <rect
                                                fill="none"
                                                height="256"
                                                stroke="none"
                                                width="256"
                                                x="0"
                                                y="0"
                                            />
                                            <path
                                                d="M32 225h12.993A4.004 4.004 0 0 0 49 220.997V138.01c0-4.976.724-5.04 1.614-.16l12.167 66.708c.397 2.177 2.516 3.942 4.713 3.942h8.512a3.937 3.937 0 0 0 3.947-4S79 127.5 80 129s14.488 52.67 14.488 52.67c.559 2.115 2.8 3.83 5.008 3.83h8.008a3.993 3.993 0 0 0 3.996-3.995v-43.506c0-4.97 1.82-5.412 4.079-.965l10.608 20.895c1.001 1.972 3.604 3.571 5.806 3.571h9.514a3.999 3.999 0 0 0 3.993-4.001v-19.49c0-4.975 2.751-6.074 6.155-2.443l6.111 6.518c1.51 1.61 4.528 2.916 6.734 2.916h7c2.21 0 5.567-.855 7.52-1.92l9.46-5.16c1.944-1.06 5.309-1.92 7.524-1.92h23.992a4.002 4.002 0 0 0 4.004-3.992v-7.516a3.996 3.996 0 0 0-4.004-3.992h-23.992c-2.211 0-5.601.823-7.564 1.834l-4.932 2.54c-4.423 2.279-12.028 3.858-16.993 3.527l2.97.198c-4.962-.33-10.942-4.12-13.356-8.467l-11.19-20.14c-1.07-1.929-3.733-3.492-5.939-3.492h-7c-2.21 0-4 1.794-4 4.001v19.49c0 4.975-1.14 5.138-2.542.382l-12.827-43.535c-.625-2.12-2.92-3.838-5.127-3.838h-8.008c-2.207 0-3.916 1.784-3.817 4.005l1.92 42.998c.221 4.969-.489 5.068-1.585.224l-15.13-66.825c-.488-2.155-2.681-3.902-4.878-3.902h-8.512a3.937 3.937 0 0 0-3.947 4s.953 77-.047 75.5s-13.937-92.072-13.937-92.072C49.252 34.758 47.21 33 45 33H31.999"
                                                fill="currentColor"
                                                fill-rule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                }
                            })}
                    </div>
                }
            })}
    }
}
