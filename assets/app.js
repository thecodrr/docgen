function search() {
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

function atTop() {
  var nav = document.getElementsByClassName("sidebar-right")[0];

  return window.scrollY <= nav.offsetTop + 50;
}

function navTouchingBottom() {
  var nav = document.getElementsByClassName("page-nav")[0];

  var height = Math.max(
    document.body.scrollHeight,
    document.documentElement.scrollHeight,
    document.body.offsetHeight,
    document.documentElement.offsetHeight,
    document.body.clientHeight,
    document.documentElement.clientHeight
  );
  // Magic number determined
  // by height of bottom wave
  return window.scrollY + nav.offsetTop + nav.offsetHeight >= height - 230;
}

function scrolledUp() {
  var height = Math.max(
    document.body.scrollHeight,
    document.documentElement.scrollHeight,
    document.body.offsetHeight,
    document.documentElement.offsetHeight,
    document.body.clientHeight,
    document.documentElement.clientHeight
  );

  // Magic number determined
  // by height of bottom wave
  return window.scrollY + window.innerHeight < height - 230;
}

function dragRightMenu() {
  if (atTop()) {
    document.getElementById("page-nav").classList.remove("fixed");
    document
      .getElementsByClassName("sidebar-right")[0]
      .classList.remove("bottom");
  } else if (scrolledUp()) {
    document.getElementById("page-nav").classList.add("fixed");
    document
      .getElementsByClassName("sidebar-right")[0]
      .classList.remove("bottom");
  } else if (navTouchingBottom()) {
    document.getElementById("page-nav").classList.remove("fixed");
    document.getElementsByClassName("sidebar-right")[0].classList.add("bottom");
  } else {
    document.getElementById("page-nav").classList.add("fixed");
    document
      .getElementsByClassName("sidebar-right")[0]
      .classList.remove("bottom");
  }
}

function isVisible(element) {
  var rect = element.getBoundingClientRect();
  var elemTop = rect.top;
  var elemBottom = rect.bottom;

  var isVisible = elemTop >= 0 && elemBottom <= window.innerHeight;
  return isVisible;
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

// Initialize mermaid.js based on color theme
var color = localStorage.getItem("docgen-color");
if (color === "dark") {
  console.log("DARK MODE");
  if ("mermaid" in globalThis) mermaid.initialize({ theme: "dark" });
} else {
  if ("mermaid" in globalThis) mermaid.initialize({ theme: "default" });
}

// Load search index
var INDEX;

fetch(BASE_PATH + "search_index.json")
  .then(function (response) {
    if (!response.ok) {
      throw new Error("HTTP error " + response.status);
    }
    return response.json();
  })
  .then(function (json) {
    INDEX = elasticlunr.Index.load(json);
    document.getElementById("search-box").oninput = search;
    search();
  });

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
    const tabId =
      ev.target.closest(`[role="tab"]`) && ev.target.closest(`[role="tab"]`).id;
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
    }
  }
};

disableScrollifMenuOpen();
dragRightMenu();
setColor();
