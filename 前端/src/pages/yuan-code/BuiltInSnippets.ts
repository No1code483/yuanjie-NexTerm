import { t } from "i18next";
/**
 * 内置代码片段库 — 对标 VSCode 内置 snippets
 *
 * 提供 Python / JavaScript / TypeScript / Rust / Go / HTML / CSS 常用片段
 * 使用 Monaco 片段语法: $1, $2, ${1:placeholder}, ${1|choice1,choice2|}, $0
 */

import { Snippet } from './SnippetStore';
export const BUILT_IN_SNIPPETS: Snippet[] = [
// ========== Python ==========
{
  id: 'builtin_py_for',
  name: t("yuan-code.BuiltInSnippets.k1"),
  prefix: 'for',
  description: t("yuan-code.BuiltInSnippets.k2"),
  body: 'for ${1:item} in ${2:iterable}:\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_if',
  name: t("yuan-code.BuiltInSnippets.k3"),
  prefix: 'if',
  description: t("yuan-code.BuiltInSnippets.k4"),
  body: 'if ${1:condition}:\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_def',
  name: t("yuan-code.BuiltInSnippets.k5"),
  prefix: 'def',
  description: t("yuan-code.BuiltInSnippets.k6"),
  body: 'def ${1:name}(${2:args}):\n\t"""${3:docstring}"""\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_class',
  name: t("yuan-code.BuiltInSnippets.k7"),
  prefix: 'class',
  description: t("yuan-code.BuiltInSnippets.k8"),
  body: 'class ${1:ClassName}(${2:object}):\n\t"""${3:docstring}"""\n\n\tdef __init__(self, ${4:args}):\n\t\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_try',
  name: 'try/except',
  prefix: 'try',
  description: t("yuan-code.BuiltInSnippets.k9"),
  body: 'try:\n\t${1:pass}\nexcept ${2:Exception} as ${3:e}:\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_main',
  name: t("yuan-code.BuiltInSnippets.k10"),
  prefix: 'main',
  description: 'if __name__ == "__main__"',
  body: 'if __name__ == "__main__":\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_with',
  name: t("yuan-code.BuiltInSnippets.k11"),
  prefix: 'with',
  description: t("yuan-code.BuiltInSnippets.k12"),
  body: 'with ${1:open}("${2:file}") as ${3:f}:\n\t${0:pass}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_lambda',
  name: t("yuan-code.BuiltInSnippets.k13"),
  prefix: 'lambda',
  description: t("yuan-code.BuiltInSnippets.k14"),
  body: 'lambda ${1:x}: ${0:x}',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_listcomp',
  name: t("yuan-code.BuiltInSnippets.k15"),
  prefix: 'lc',
  description: t("yuan-code.BuiltInSnippets.k15"),
  body: '[${1:expr} for ${2:item} in ${3:iterable}]',
  scope: 'python',
  isBuiltIn: true
}, {
  id: 'builtin_py_dictcomp',
  name: t("yuan-code.BuiltInSnippets.k16"),
  prefix: 'dc',
  description: t("yuan-code.BuiltInSnippets.k16"),
  body: '{${1:key}: ${2:value} for ${3:item} in ${4:iterable}}',
  scope: 'python',
  isBuiltIn: true
},
// ========== JavaScript ==========
{
  id: 'builtin_js_fn',
  name: t("yuan-code.BuiltInSnippets.k17"),
  prefix: 'fn',
  description: t("yuan-code.BuiltInSnippets.k18"),
  body: 'function ${1:name}(${2:params}) {\n\t${0}\n}',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_arrow',
  name: t("yuan-code.BuiltInSnippets.k19"),
  prefix: 'af',
  description: t("yuan-code.BuiltInSnippets.k19"),
  body: 'const ${1:name} = (${2:params}) => {\n\t${0}\n}',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_clog',
  name: t("yuan-code.BuiltInSnippets.k20"),
  prefix: 'cl',
  description: 'console.log',
  body: 'console.log(${1:value})$0',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_ife',
  name: t("yuan-code.BuiltInSnippets.k21"),
  prefix: 'ife',
  description: t("yuan-code.BuiltInSnippets.k22"),
  body: '(() => {\n\t${0}\n})()',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_for',
  name: t("yuan-code.BuiltInSnippets.k1"),
  prefix: 'for',
  description: t("yuan-code.BuiltInSnippets.k1"),
  body: 'for (let ${1:i} = 0; ${1:i} < ${2:len}; ${1:i}++) {\n\t${0}\n}',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_foreach',
  name: t("yuan-code.BuiltInSnippets.k23"),
  prefix: 'fe',
  description: t("yuan-code.BuiltInSnippets.k24"),
  body: '${1:arr}.forEach((${2:item}) => {\n\t${0}\n})',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_map',
  name: t("yuan-code.BuiltInSnippets.k25"),
  prefix: 'map',
  description: t("yuan-code.BuiltInSnippets.k26"),
  body: '${1:arr}.map((${2:item}) => ${0:item})',
  scope: 'javascript',
  isBuiltIn: true
}, {
  id: 'builtin_js_imp',
  name: t("yuan-code.BuiltInSnippets.k27"),
  prefix: 'im',
  description: t("yuan-code.BuiltInSnippets.k28"),
  body: 'import { ${1:name} } from "${2:module}"$0',
  scope: 'javascript',
  isBuiltIn: true
},
// ========== TypeScript ==========
{
  id: 'builtin_ts_interface',
  name: t("yuan-code.BuiltInSnippets.k29"),
  prefix: 'interface',
  description: t("yuan-code.BuiltInSnippets.k30"),
  body: 'interface ${1:Name} {\n\t${2:key}: ${3:type}\n}$0',
  scope: 'typescript',
  isBuiltIn: true
}, {
  id: 'builtin_ts_type',
  name: t("yuan-code.BuiltInSnippets.k31"),
  prefix: 'type',
  description: t("yuan-code.BuiltInSnippets.k32"),
  body: 'type ${1:Name} = ${2:type}$0',
  scope: 'typescript',
  isBuiltIn: true
}, {
  id: 'builtin_ts_enum',
  name: t("yuan-code.BuiltInSnippets.k33"),
  prefix: 'enum',
  description: t("yuan-code.BuiltInSnippets.k34"),
  body: 'enum ${1:Name} {\n\t${2:Key} = ${3:value}\n}$0',
  scope: 'typescript',
  isBuiltIn: true
},
// ========== Rust ==========
{
  id: 'builtin_rs_fn',
  name: t("yuan-code.BuiltInSnippets.k5"),
  prefix: 'fn',
  description: t("yuan-code.BuiltInSnippets.k35"),
  body: 'fn ${1:name}(${2:args}) -> ${3:ReturnType} {\n\t${0}\n}',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_struct',
  name: t("yuan-code.BuiltInSnippets.k36"),
  prefix: 'struct',
  description: t("yuan-code.BuiltInSnippets.k37"),
  body: 'struct ${1:Name} {\n\t${2:field}: ${3:Type}\n}$0',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_impl',
  name: t("yuan-code.BuiltInSnippets.k38"),
  prefix: 'impl',
  description: t("yuan-code.BuiltInSnippets.k39"),
  body: 'impl ${1:Type} {\n\tfn ${2:name}(&self) {\n\t\t${0}\n\t}\n}',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_enum',
  name: t("yuan-code.BuiltInSnippets.k40"),
  prefix: 'enum',
  description: t("yuan-code.BuiltInSnippets.k41"),
  body: 'enum ${1:Name} {\n\t${2:Variant1},\n\t${3:Variant2},\n}$0',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_match',
  name: t("yuan-code.BuiltInSnippets.k42"),
  prefix: 'match',
  description: t("yuan-code.BuiltInSnippets.k43"),
  body: 'match ${1:expr} {\n\t${2:pattern} => ${3:result},\n\t_ => ${0:default},\n}',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_let',
  name: t("yuan-code.BuiltInSnippets.k44"),
  prefix: 'let',
  description: t("yuan-code.BuiltInSnippets.k45"),
  body: 'let ${1:name} = ${0:value};',
  scope: 'rust',
  isBuiltIn: true
}, {
  id: 'builtin_rs_print',
  name: 'println!',
  prefix: 'println',
  description: t("yuan-code.BuiltInSnippets.k46"),
  body: 'println!("${1:format}", ${2:args});$0',
  scope: 'rust',
  isBuiltIn: true
},
// ========== Go ==========
{
  id: 'builtin_go_fn',
  name: t("yuan-code.BuiltInSnippets.k5"),
  prefix: 'func',
  description: t("yuan-code.BuiltInSnippets.k47"),
  body: 'func ${1:name}(${2:args}) ${3:ReturnType} {\n\t${0}\n}',
  scope: 'go',
  isBuiltIn: true
}, {
  id: 'builtin_go_struct',
  name: t("yuan-code.BuiltInSnippets.k36"),
  prefix: 'struct',
  description: t("yuan-code.BuiltInSnippets.k48"),
  body: 'type ${1:Name} struct {\n\t${2:Field} ${3:Type}\n}$0',
  scope: 'go',
  isBuiltIn: true
}, {
  id: 'builtin_go_error',
  name: t("yuan-code.BuiltInSnippets.k49"),
  prefix: 'iferr',
  description: t("yuan-code.BuiltInSnippets.k50"),
  body: 'if err != nil {\n\t${0:return err}\n}',
  scope: 'go',
  isBuiltIn: true
}, {
  id: 'builtin_go_for',
  name: t("yuan-code.BuiltInSnippets.k1"),
  prefix: 'for',
  description: t("yuan-code.BuiltInSnippets.k51"),
  body: 'for ${1:i} := 0; ${1:i} < ${2:n}; ${1:i}++ {\n\t${0}\n}',
  scope: 'go',
  isBuiltIn: true
}, {
  id: 'builtin_go_range',
  name: t("yuan-code.BuiltInSnippets.k52"),
  prefix: 'range',
  description: t("yuan-code.BuiltInSnippets.k53"),
  body: 'for ${1:i}, ${2:v} := range ${3:slice} {\n\t${0}\n}',
  scope: 'go',
  isBuiltIn: true
},
// ========== HTML ==========
{
  id: 'builtin_html_div',
  name: t("yuan-code.BuiltInSnippets.k54"),
  prefix: 'div',
  description: t("yuan-code.BuiltInSnippets.k55"),
  body: '<div class="${1:className}">\n\t${0}\n</div>',
  scope: 'html',
  isBuiltIn: true
}, {
  id: 'builtin_html_input',
  name: t("yuan-code.BuiltInSnippets.k56"),
  prefix: 'input',
  description: t("yuan-code.BuiltInSnippets.k57"),
  body: '<input type="${1:text}" placeholder="${2:placeholder}" />$0',
  scope: 'html',
  isBuiltIn: true
}, {
  id: 'builtin_html_a',
  name: t("yuan-code.BuiltInSnippets.k58"),
  prefix: 'a',
  description: t("yuan-code.BuiltInSnippets.k59"),
  body: '<a href="${1:url}">${2:text}</a>$0',
  scope: 'html',
  isBuiltIn: true
},
// ========== CSS ==========
{
  id: 'builtin_css_flex',
  name: t("yuan-code.BuiltInSnippets.k60"),
  prefix: 'flex',
  description: t("yuan-code.BuiltInSnippets.k61"),
  body: 'display: flex;\njustify-content: ${1:center};\nalign-items: ${2:center};$0',
  scope: 'css',
  isBuiltIn: true
}, {
  id: 'builtin_css_grid',
  name: t("yuan-code.BuiltInSnippets.k62"),
  prefix: 'grid',
  description: t("yuan-code.BuiltInSnippets.k63"),
  body: 'display: grid;\ngrid-template-columns: ${1:repeat(3, 1fr)};\ngap: ${2:16px};$0',
  scope: 'css',
  isBuiltIn: true
}, {
  id: 'builtin_css_margin',
  name: 'margin',
  prefix: 'm',
  description: 'CSS margin',
  body: 'margin: ${1:0} ${2:0} ${3:0} ${4:0};$0',
  scope: 'css',
  isBuiltIn: true
}, {
  id: 'builtin_css_padding',
  name: 'padding',
  prefix: 'p',
  description: 'CSS padding',
  body: 'padding: ${1:0} ${2:0} ${3:0} ${4:0};$0',
  scope: 'css',
  isBuiltIn: true
}];