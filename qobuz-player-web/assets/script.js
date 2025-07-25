let evtSource;

function initSse() {
  evtSource = new EventSource("/sse");

  evtSource.addEventListener("status", (_event) => {
    const elements = document.querySelectorAll("[data-sse=status]");

    for (const element of elements) {
      htmx.trigger(element, "status");
    }
  });

  evtSource.addEventListener("tracklist", (_event) => {
    const elements = document.querySelectorAll("[data-sse=tracklist]");

    for (const element of elements) {
      htmx.trigger(element, "tracklist");
    }
  });

  evtSource.addEventListener("volume", (event) => {
    const slider = document.getElementById("volume-slider");
    if (slider === null) {
      return;
    }
    slider.value = event.data;
  });

  evtSource.addEventListener("position", (event) => {
    const slider = document.getElementById("progress-slider");
    if (slider === null) {
      return;
    }
    slider.value = event.data;

    const positionElement = document.getElementById("position");

    const minutes = Math.floor(event.data / 60)
      .toString()
      .padStart(2, "0");
    const seconds = (event.data % 60).toString().padStart(2, "0");

    positionElement.innerText = `${minutes}:${seconds}`;
  });
}

initSse();

function refreshSse() {
  const elements = document.querySelectorAll("[hx-trigger='tracklist'");

  for (const element of elements) {
    htmx.trigger(element, "tracklist");
  }

  const statusElements = document.querySelectorAll("[hx-trigger='status'");

  for (const element of statusElements) {
    htmx.trigger(element, "status");
  }
}

document.addEventListener("visibilitychange", () => {
  if (!document.hidden) {
    initSse();
    refreshSse();
  }
});

function focusSearchInput() {
  document.getElementById("query").focus();
}

function loadSearchInput() {
  let value = sessionStorage.getItem("search-query");
  document.getElementById("query").value = value;
}

function setSearchQuery(value) {
  sessionStorage.setItem("search-query", value);
}
