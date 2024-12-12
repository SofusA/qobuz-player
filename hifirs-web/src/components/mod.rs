use leptos::{component, prelude::*, IntoView};

use crate::{html, icons::Star};
pub mod list;

#[component]
pub fn toggle_favorite(id: String, is_favorite: bool) -> impl IntoView {
    html! {
        <button
            class="flex gap-2 items-center py-2 px-4 bg-blue-500 rounded"
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
pub fn info(hires_available: bool, explicit: bool) -> impl IntoView {
    html! {
        <div class="text-gray-400 whitespace-nowrap">

            {if explicit {
                Some(
                    html! {
                        <svg
                            class="inline-block"
                            height="24"
                            viewBox="0 0 24 24"
                            width="24"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path
                                d="M21 3H3v18h18V3zm-6 6h-4v2h4v2h-4v2h4v2H9V7h6v2z"
                                fill="currentColor"
                            />
                        </svg>
                    },
                )
            } else {
                None
            }}
            {if hires_available {
                Some(
                    html! {
                        <svg
                            class="inline-block"
                            height="24"
                            viewBox="0 0 256 256"
                            width="24"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <rect fill="none" height="256" stroke="none" width="256" x="0" y="0" />
                            <path
                                d="M32 225h12.993A4.004 4.004 0 0 0 49 220.997V138.01c0-4.976.724-5.04 1.614-.16l12.167 66.708c.397 2.177 2.516 3.942 4.713 3.942h8.512a3.937 3.937 0 0 0 3.947-4S79 127.5 80 129s14.488 52.67 14.488 52.67c.559 2.115 2.8 3.83 5.008 3.83h8.008a3.993 3.993 0 0 0 3.996-3.995v-43.506c0-4.97 1.82-5.412 4.079-.965l10.608 20.895c1.001 1.972 3.604 3.571 5.806 3.571h9.514a3.999 3.999 0 0 0 3.993-4.001v-19.49c0-4.975 2.751-6.074 6.155-2.443l6.111 6.518c1.51 1.61 4.528 2.916 6.734 2.916h7c2.21 0 5.567-.855 7.52-1.92l9.46-5.16c1.944-1.06 5.309-1.92 7.524-1.92h23.992a4.002 4.002 0 0 0 4.004-3.992v-7.516a3.996 3.996 0 0 0-4.004-3.992h-23.992c-2.211 0-5.601.823-7.564 1.834l-4.932 2.54c-4.423 2.279-12.028 3.858-16.993 3.527l2.97.198c-4.962-.33-10.942-4.12-13.356-8.467l-11.19-20.14c-1.07-1.929-3.733-3.492-5.939-3.492h-7c-2.21 0-4 1.794-4 4.001v19.49c0 4.975-1.14 5.138-2.542.382l-12.827-43.535c-.625-2.12-2.92-3.838-5.127-3.838h-8.008c-2.207 0-3.916 1.784-3.817 4.005l1.92 42.998c.221 4.969-.489 5.068-1.585.224l-15.13-66.825c-.488-2.155-2.681-3.902-4.878-3.902h-8.512a3.937 3.937 0 0 0-3.947 4s.953 77-.047 75.5s-13.937-92.072-13.937-92.072C49.252 34.758 47.21 33 45 33H31.999"
                                fill="currentColor"
                                fill-rule="evenodd"
                            />
                        </svg>
                    },
                )
            } else {
                None
            }}
        </div>
    }
}
