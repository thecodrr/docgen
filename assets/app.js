async function search() {
  await loadSearchIndex();

  box = document.getElementById("search-box");
  list = document.getElementById("search-results");
  list.innerHTML = "";

  if (box.value == "") {
    return;
  }

  config = {
    fields: {
      title: {
        boost: 2,
      },
      body: {
        boost: 1,
      },
    },
    bool: "OR",
    expand: true,
  };

  INDEX.search(box.value, config).forEach(function (result) {
    const listItem = document.createElement("li");
    listItem.className = "search-result-item";

    const anchor = document.createElement("a");
    anchor.href = result.doc.uri;

    const preview = document.createElement("p");
    preview.classList.add("search-result-item-preview");
    preview.innerHTML = searchPreview(result.doc.preview);
    preview.innerHTML = preview.innerText;

    anchor.appendChild(document.createTextNode(result.doc.title));
    anchor.appendChild(preview);
    listItem.appendChild(anchor);

    list.appendChild(listItem);
  });
}

function searchPreview(body) {
  return (
    body
      .replace(/=+/g, "")
      .replace(/#+/g, "")
      .replace(/\*+/g, "")
      .replace(/_+/g, "") + "..."
  );
}

function disableScrollifMenuOpen() {
  var checkbox = document.getElementById("menu-toggle-switch");

  if (checkbox.checked) {
    document.body.style.overflow = "hidden";
  } else {
    document.body.style.overflow = "auto";
  }
}

function toggleColor() {
  var color = localStorage.getItem("docgen-color");

  if (color === "dark") {
    localStorage.setItem("docgen-color", "light");
  } else {
    localStorage.setItem("docgen-color", "dark");
  }

  setColor();
}

document
  .getElementById("light-dark-mode-switch")
  .addEventListener("click", toggleColor);

function initMermaid() {
  const mermaidScript = document.getElementById("mermaid.min.js");
  if (!mermaidScript) return;

  mermaidScript.onload = function () {
    // Initialize mermaid.js based on color theme
    var color = localStorage.getItem("docgen-color");
    if (color === "dark") {
      if ("mermaid" in globalThis) mermaid.initialize({ theme: "dark" });
    } else {
      if ("mermaid" in globalThis) mermaid.initialize({ theme: "dark" });
    }
  };
}

// Load search index
var INDEX;

async function loadSearchIndex() {
  if (INDEX) return;

  const response = await fetch(BASE_PATH + "search_index.json");

  if (!response.ok) {
    throw new Error("HTTP error " + response.status);
  }
  const json = await response.json();

  INDEX = elasticlunr.Index.load(json);
}

document.getElementById("search-box").oninput = search;
// Setup keyboard shortcuts
document.onkeydown = function (e) {
  var searchResults = document.getElementById("search-results");
  var first = searchResults.firstChild;
  var searchBox = document.getElementById("search-box");

  switch (e.keyCode) {
    case 83: // The S key
      if (document.activeElement == searchBox) {
        break;
      } else {
        searchBox.focus();
        e.preventDefault();
      }
      break;
    case 38: // if the UP key is pressed
      if (document.activeElement == (searchBox || first)) {
        break;
      } else {
        document.activeElement.parentNode.previousSibling.firstChild.focus();
        e.preventDefault();
      }
      break;
    case 40: // if the DOWN key is pressed
      if (document.activeElement == searchBox) {
        first.firstChild.focus();
        e.preventDefault();
      } else {
        document.activeElement.parentNode.nextSibling.firstChild.focus();
        e.preventDefault();
      }
      break;
    case 27: // if the ESC key is pressed
      if (first) {
        searchResults.innerHTML = "";
      }
      break;
  }
};

document.onclick = (ev) => {
  if (ev.target instanceof HTMLElement) {
    const clickedTab = ev.target.closest(`[role="tab"]`);
    const tabId = clickedTab && clickedTab.id;

    if (tabId) {
      for (const tabItem of document.querySelectorAll(
        `.tabgroup label#${tabId}`
      )) {
        const tabGroup = tabItem.closest(".tabgroup");

        const activeTab = tabGroup.querySelector(`label.active[role="tab"]`);

        activeTab.classList.remove("active");
        tabItem.classList.add("active");

        const activeTabPanel = tabGroup.querySelector(`div.tab-panel.active`);
        const tabPanel = tabGroup.querySelector(
          `.tab-panel[data-tab-id="${tabId}"]`
        );

        activeTabPanel.classList.remove("active");
        tabPanel.classList.add("active");
      }

      setTimeout(() =>
        clickedTab.scrollIntoView({
          block: "nearest",
          inline: "nearest",
        })
      );
    }
  }
};

disableScrollifMenuOpen();
setColor();
initMermaid();
