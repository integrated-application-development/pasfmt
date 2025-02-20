/*
Extends the provided pascal language definition.
  * add support for the alternative multiline command syntax: `(* *)`
  * add support for hex integer literals of more lengths
  * add support for underscores in hex integer literals
  * add support for binary integer literals
  * add support for hex and binary character literals
  * add support for multiline strings
  * remove | and % from operators
  * remove treatment of < and > as matched brackets (it's not always generics)
*/

import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import * as pascal from "monaco-editor/esm/vs/basic-languages/pascal/pascal";

monaco.languages.register({ id: "delphi" });

const lang: monaco.languages.IMonarchLanguage = structuredClone(
  pascal.language
);

lang.includeLF = true;

// alt comments
lang.tokenizer.altcomment = [
  [/[^)*]+/, "comment"],
  [/\*\)/, "comment", "@pop"],
  [/[)*]/, "comment"],
];
lang.tokenizer.whitespace.push([/\(\*/, "comment", "@altcomment"]);

// all of these use unshift to have higher precedence that the existing rules

// binary integer literals
lang.tokenizer.root.unshift([/%[01_]*/, "number.hex"]);
// more general hex integer
lang.tokenizer.root.unshift([/\$[0-9a-fA-F_]*/, "number.hex"]);
// more general hex character literals
lang.tokenizer.root.unshift([/#\$[0-9a-fA-F_]*/, "string"]);
// binary character literals
lang.tokenizer.root.unshift([/#%[01_]*/, "string"]);

// multiline strings
lang.tokenizer.root.unshift([
  /('(?:'')+)\r?\n/,
  { token: "string", next: "@multilinestring.$1" },
]);
lang.tokenizer.multilinestring = [
  [
    /('(?:'')+)/,
    {
      cases: {
        "$1==$S2": { token: "string", next: "@pop" },
        "@default": "string",
      },
    },
  ],
  [/./, "string"],
];

// removed | %
lang.symbols = /[=><:@^&+\-*\/]+/;

lang.operators.splice(lang.operators.indexOf("%"), 1);

// <> are configured to be matched, as generics.
// This completely overlooks the match that much of the time it's actually
// just a binary operator. It takes a lot of work to undo this mistake.

lang.brackets = [
  // used for comments
  // { open: "{", close: "}", token: "delimiter.curly" },
  { open: "[", close: "]", token: "delimiter.square" },
  { open: "(", close: ")", token: "delimiter.parenthesis" },
  // { open: "<", close: ">", token: "delimiter.angle" },
];

let toRemove = lang.tokenizer.root.findIndex(
  (elm) =>
    Array.isArray(elm) && elm[0].toString() == /[<>](?!@symbols)/.toString()
);
lang.tokenizer.root.splice(toRemove, 1);

const conf = structuredClone(pascal.conf);

conf.autoClosingPairs = [
  { open: "{", close: "}" },
  { open: "[", close: "]" },
  { open: "(", close: ")" },
  // { open: "<", close: ">" },
  { open: "'", close: "'" },
];

conf.surroundingPairs = [
  { open: "{", close: "}" },
  { open: "[", close: "]" },
  { open: "(", close: ")" },
  // { open: "<", close: ">" },
  { open: "'", close: "'" },
];

conf.brackets = [
  ["{", "}"],
  ["[", "]"],
  ["(", ")"],
  // ["<", ">"]
];

export { lang, conf };
