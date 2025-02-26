# Configuration

Some aspects of formatting style are configurable.

Configuration options can either be specified in a `pasfmt.toml` configuration file, or on the command line.

## Available Options

<table>
  <thead>
    <tr>
      <th>Name</th>
      <th>Valid Values</th>
      <th>Default</th>
      <th>Description</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>wrap_column</td>
      <td>&lt;unsigned&nbsp;integer&gt;</td>
      <td>120</td>
      <td>Target line length before wrapping</td>
    </tr>
    <tr>
      <td>begin_style</td>
      <td>"auto", "always_wrap"</td>
      <td>"auto"</td>
      <td>
        Places the <code>begin</code> after control flow statements (e.g. <code>if</code>).<br>
        If "always_wrap", the <code>begin</code> will always be placed on the next line at the
        same indentation as the statement it is within.
      </td>
    </tr>
    <tr>
      <td>encoding</td>
      <td>"native", &lt;NAME&gt;</td>
      <td>"native"</td>
      <td>
        The encoding to use when reading and writing files.<br />
        If "native":
        <ul>
          <li>on Windows, the system ANSI codepage is used</li>
          <li>
            otherwise, UTF-8 is used
          </li>
        </ul>
        In all cases a detected BOM will override the configured encoding.
      </td>
    </tr>
    <tr>
      <td>use_tabs</td>
      <td>&lt;boolean&gt;</td>
      <td>false</td>
      <td>Use tab characters for indentation</td>
    </tr>
    <tr>
      <td>tab_width</td>
      <td>&lt;unsigned&nbsp;integer&gt;</td>
      <td>2</td>
      <td>Number of spaces per indentation (ignored if use_tabs=true)</td>
    </tr>
    <tr>
      <td>continuation_indents</td>
      <td>&lt;unsigned&nbsp;integer&gt;</td>
      <td>2</td>
      <td>
        Width of continuations, measured as a multiple of the configured
        indentation. Continuations are used to further indent the wrapped lines
        from a "logical line". Indentations are used to indent the base of a
        "logical line".
      </td>
    </tr>
    <tr>
      <td>line_ending</td>
      <td>"lf", "crlf", "native"</td>
      <td>"native"</td>
      <td>
        Line ending character sequence.<br />
        If "native":
        <ul>
          <li>on Windows, "crlf" is used</li>
          <li>otherwise, "lf" is used</li>
        </ul>
      </td>
    </tr>
  </tbody>
</table>
