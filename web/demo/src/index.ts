import init, { fmt, default_settings_toml, SettingsWrapper } from "../../pkg";

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

await init();

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
const resetSettings = () => settingsEditor.setValue(default_settings_toml());
resetSettings();
resetDefaultSettingsButton.onclick = resetSettings;

const parseSettings = () => {
  try {
    return new SettingsWrapper(settingsEditor.getValue());
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

// Start with the diff editor loaded
var diffEditor = createDiffEditor();
let isDiffView = true;

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
    formattedModel.setValue(fmt(originalModel.getValue(), settingsObj));
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

const url = new URL(window.location.href);
let source = url.searchParams.get("source");
let settings = url.searchParams.get("settings");

if (source !== null) {
  let decoded = atob(source);
  originalEditor.setValue(decoded);
} else {
  loadSampleFile("/examples/simple.pas");
}

if (settings !== null) {
  let decoded = atob(settings);
  settingsEditor.setValue(decoded);
}

const shareExample = document.getElementById(
  "share-example"
) as HTMLButtonElement;
shareExample.onclick = () => {
  url.searchParams.set("source", btoa(originalEditor.getValue()));
  url.searchParams.set("settings", btoa(settingsEditor.getValue()));
  window.history.replaceState(null, "", url);
  navigator.clipboard.writeText(window.location.href);
};

formatEditors();
