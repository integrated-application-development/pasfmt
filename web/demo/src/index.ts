import * as monaco from "monaco-editor/esm/vs/editor/editor.api";

// support for ini, good enough for our TOML config
import "monaco-editor/esm/vs/basic-languages/ini/ini.contribution";

// extra editor features
// a subset of the imports from 'monaco-editor/esm/vs/editor/editor.all.js'
import "monaco-editor/esm/vs/editor/contrib/readOnlyMessage/browser/contribution.js";
import "monaco-editor/esm/vs/editor/browser/widget/diffEditor/diffEditor.contribution.js";
import "monaco-editor/esm/vs/editor/contrib/diffEditorBreadcrumbs/browser/contribution.js";
import "monaco-editor/esm/vs/editor/contrib/find/browser/findController.js";
import "monaco-editor/esm/vs/editor/contrib/folding/browser/folding.js";
import "monaco-editor/esm/vs/editor/contrib/fontZoom/browser/fontZoom.js";
import "monaco-editor/esm/vs/editor/contrib/inlineEdit/browser/inlineEdit.contribution.js";
import "monaco-editor/esm/vs/editor/contrib/inlineEdits/browser/inlineEdits.contribution.js";
import "monaco-editor/esm/vs/editor/contrib/hover/browser/hoverContribution.js";
import "monaco-editor/esm/vs/editor/contrib/wordOperations/browser/wordOperations.js";

// custom delphi tokenizer
import * as delphi from "./delphi";

const BASE_URL = import.meta.env.BASE_URL;

const url = new URL(window.location.href);
const version_param = url.searchParams.get("version");
const source_param = url.searchParams.get("source");
const settings_param = url.searchParams.get("settings");

// All use of this needs to be compatible with all supported versions of the WASM module.
let pasfmt: any;

// Load a specific version
const loadWasmVersion = async (version: string) => {
  // Dynamically import the JS glue code
  const mod = import.meta.env.DEV
    ? await import("../../pkg/web.js")
    : await import(/* @vite-ignore */ `${BASE_URL}pkg/${version}/web.js`);

  await mod.default();

  console.log(`Loaded WASM module version: ${version}`);

  pasfmt = mod;
};

const loadVersion = async () => {
  await loadWasmVersion(versionPicker.value);
};

const versionPicker = document.getElementById(
  "version-picker"
)! as HTMLSelectElement;

await fetch(`${BASE_URL}versions.json`)
  .then((res) => res.json())
  .then((versions: Array<string>) => {
    versions.forEach((v) => {
      const opt = document.createElement("option");
      opt.value = v;
      opt.textContent = v;
      versionPicker.appendChild(opt);
    });

    if (version_param !== null) {
      const decoded = atob(version_param);
      versionPicker.value = decoded;
      if (!versionPicker.value) {
        console.log(`Invalid version from search parameters: ${decoded}`);
      }
    } else {
      versionPicker.value = versions[0];
    }
  })
  .then(loadVersion);

const showLoadingSpinner = () => {
  versionPicker.disabled = true;
  versionPicker.classList.add('loading')
}

const hideLoadingSpinner = () => {
  versionPicker.disabled = false;
  versionPicker.classList.remove('loading')
}

versionPicker.addEventListener("change", async () => {
  showLoadingSpinner();
  await loadVersion();
  hideLoadingSpinner();
  updateSetingsOnVersionChange();
  formatEditors();
});

const updateSetingsOnVersionChange = () => {
  const new_valid_keys = defaultSettings()
    .split("\n")
    .map((line) => line.split("=")[0].trim());
  const new_settings = settingsEditor
    .getValue()
    .split("\n")
    .map((line) => {
      if (!line.includes("=")) {
        return line;
      }

      const commented_out = "#(unavailable)";
      if (line.startsWith(commented_out)) {
        line = line.substring(commented_out.length);
      }

      let key = line.split("=")[0].trim();
      if (new_valid_keys.includes(key)) {
        return line;
      } else {
        return commented_out + line;
      }
    })
    .join("\n");

  settingsEditor.setValue(new_settings);
};

const diffEditorContainer = document.getElementById("diffpane")!;
const sideBySideContainer = document.getElementById("editpane")!;

monaco.languages.setMonarchTokensProvider("delphi", delphi.lang);
monaco.languages.setLanguageConfiguration("delphi", delphi.conf);

const originalModel = monaco.editor.createModel("", "delphi");
const formattedModel = monaco.editor.createModel("", "delphi");

const makeRuler = (col: number): monaco.editor.IRulerOption => ({
  column: col,
  color: null,
});

// It's actually TOML, but ini is close enough.
// A PR is open to add TOML support https://github.com/microsoft/monaco-editor/pull/4786.
const settingsModel = monaco.editor.createModel("", "ini");
const settingsDiv = document.getElementById("settingsEditor")!;
const settingsEditor = monaco.editor.create(settingsDiv, {
  model: settingsModel,
  automaticLayout: true,
  readOnly: false,
  minimap: { enabled: false },
  scrollbar: { vertical: "hidden", horizontal: "hidden" },
});

const resetDefaultSettingsButton = document.getElementById(
  "resetToDefaultSettings"
)!;
const defaultSettings = (): string => pasfmt.default_settings_toml();
const resetSettings = () => settingsEditor.setValue(defaultSettings());
resetSettings();
resetDefaultSettingsButton.onclick = resetSettings;

const parseSettings = () => {
  try {
    return new pasfmt.SettingsWrapper(settingsEditor.getValue());
  } catch (error) {
    throw new Error("Failed to parse settings", {
      cause: error,
    });
  }
};

const closeModalButton = document.getElementById(
  "closeModal"
) as HTMLButtonElement;

const setSettingsBorderCol = (col) =>
  document.documentElement.style.setProperty("--settings-border", col);

let settingsErrorTimeout;
let settingsValid = true;
settingsModel.onDidChangeContent(() => {
  clearTimeout(settingsErrorTimeout);

  try {
    parseSettings();

    settingsValid = true;
    closeModalButton.disabled = false;
    setSettingsBorderCol("currentColor");
    clearErrorsInModel(settingsModel);
  } catch (error) {
    settingsValid = false;
    closeModalButton.disabled = true;
    settingsErrorTimeout = setTimeout(() => {
      setSettingsBorderCol("red");
      renderErrorInModel(error, settingsModel);
    }, 200);
  }
});

const modal = document.getElementById("settingsModal")!;
const openSettingsButton = document.getElementById("openSettings")!;

// Open the modal
openSettingsButton.onclick = () => (modal.style.display = "flex");

const closeSettings = () => {
  modal.style.display = "none";
  formatEditors();
};

const closeSettingsIfValid = () => {
  if (settingsValid) closeSettings();
};

closeModalButton.onclick = closeSettingsIfValid;

window.addEventListener("click", (event) => {
  if (event.target == modal) {
    closeSettingsIfValid();
  }
});

const originalEditorDiv = document.getElementById("original-editor")!;
const formattedEditorDiv = document.getElementById("formatted-editor")!;

const originalEditor = monaco.editor.create(originalEditorDiv, {
  model: originalModel,
  automaticLayout: true,
  readOnly: false,
});

const formattedEditor = monaco.editor.create(formattedEditorDiv, {
  model: formattedModel,
  automaticLayout: true,
  readOnly: true,
  renderValidationDecorations: "on",
});

const createDiffEditor = (): monaco.editor.IStandaloneDiffEditor => {
  let diffEditor = monaco.editor.createDiffEditor(diffEditorContainer, {
    automaticLayout: true,
    originalEditable: true,
    readOnly: true,
    ignoreTrimWhitespace: false,
  });
  diffEditor.setModel({
    original: originalModel,
    modified: formattedModel,
  });
  return diffEditor;
};

// Start with the diff editor loaded, but not active
var diffEditor = createDiffEditor();
let isDiffView = false;

const renderEditors = () => {
  if (isDiffView) {
    sideBySideContainer.style.display = "none";
    diffEditorContainer.style.display = "block";
    diffEditor.layout();
  } else {
    diffEditorContainer.style.display = "none";
    sideBySideContainer.style.display = "flex";
    originalEditor.layout();
    formattedEditor.layout();
  }
};
renderEditors();

const toggleDiffView = () => {
  isDiffView = !isDiffView;
  renderEditors();
};

document
  .getElementById("toggle-view")!
  .addEventListener("click", toggleDiffView);

const renderErrorInModel = (error, model) => {
  const errorMessage =
    error + (error.cause ? `\nCaused by: ${error.cause}` : "");
  const markers = [
    {
      startLineNumber: 1,
      startColumn: 1,
      endLineNumber: model.getLineCount(),
      endColumn: 1,
      message: errorMessage,
      severity: monaco.MarkerSeverity.Error,
    },
  ];

  monaco.editor.setModelMarkers(model, "", markers);
};

const clearErrorsInModel = (model) => {
  monaco.editor.setModelMarkers(model, "", []);
};

const updateRulers = (maxLineLen) => {
  originalEditor.updateOptions({ rulers: [makeRuler(maxLineLen)] });
  formattedEditor.updateOptions({ rulers: [makeRuler(maxLineLen)] });
  if (diffEditor !== undefined) {
    diffEditor.updateOptions({ rulers: [makeRuler(maxLineLen)] });
  }
};

const formatEditors = () => {
  try {
    let settingsObj = parseSettings();
    updateRulers(settingsObj.max_line_len());
    formattedModel.setValue(pasfmt.fmt(originalModel.getValue(), settingsObj));
  } catch (error) {
    console.log(error);
    renderErrorInModel(error, formattedModel);
  }
};
originalModel.onDidChangeContent(formatEditors);

const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
const updateTheme = () => {
  monaco.editor.setTheme(prefersDark.matches ? "vs-dark" : "vs");
};
updateTheme();
prefersDark.onchange = updateTheme;

const samplePicker = document.getElementById(
  "sample-picker"
) as HTMLSelectElement;

const loadSampleFile = async (sampleFile: string) => {
  originalModel.setValue(await fetch(sampleFile).then((resp) => resp.text()));
};

const loadSample = async () => {
  var sampleFile = samplePicker.value;
  samplePicker.value = "";
  if (sampleFile) {
    loadSampleFile(sampleFile);
  } else {
    originalModel.setValue("");
  }
  formatEditors();
};
document
  .getElementById("sample-picker")!
  .addEventListener("change", loadSample);

if (source_param !== null) {
  let decoded = atob(source_param);
  originalEditor.setValue(decoded);
} else {
  loadSampleFile(`${BASE_URL}examples/simple.pas`);
}

if (settings_param !== null) {
  let decoded = atob(settings_param);
  settingsEditor.setValue(decoded);
}

const shareExample = document.getElementById(
  "share-example"
) as HTMLButtonElement;
shareExample.onclick = () => {
  url.searchParams.set("source", btoa(originalEditor.getValue()));
  url.searchParams.set("settings", btoa(settingsEditor.getValue()));
  url.searchParams.set("version", btoa(versionPicker.value));
  window.history.replaceState(null, "", url);
  navigator.clipboard.writeText(window.location.href);
};

formatEditors();
