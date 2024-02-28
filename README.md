# swc-plugin-react-css-modules

> [!WARNING]  
> Plugin is a work in progress with no guarantee of release

SWC plugin for advanced CSS modules support in React:

It transforms styleName attribute of JSX components into className using compile-time CSS module resolution, allowing for a cleaner use of CSS modules in React.

## Usage Examples

Assuming `style.css` in the following examples is compiled as CSS Module.

**Without this plugin**
```jsx
import styles from './styles.css';

export default function Component() {
  return (
    <div className={styles.container}>
      <h1 className={styles.title}>Example</div>
      <p styleName={styles.text}>Sample text paragraph.</p>
      <p className={`${styles.text} ${styles.special}`}>
        Sample text paragraph with special style.
      </p>
    </div>
  );
}
```

**With this plugin**
```jsx
import './styles.css';

export default function Component() {
  return (
    <div styleName="container">
      <h1 styleName="title">Example</div>
      <p styleName="text">Sample text paragraph.</p>
      <p styleName="text special">
        Sample text paragraph with special style.
      </p>
    </div>
  );
}
```

If multiple files, `styles-01.css` and `styles-02.css` contain styles with the same
names, thus causing confusion on which style to select, this plugin allows explicit
stylesheet prefixes:
```jsx
import styles1 from './styles-01.css';
import styles2 from './styles-02.css';

export default function Component() {
  return (
    <div styleName="styles1.container">
      <h1 styleName="styles1.title">Example</div>
      <p styleName="styles1.text">Sample text paragraph.</p>
      <p styleName="styles1.text styles2.special">
        Sample text paragraph with special style.
      </p>
    </div>
  );
}
```

**With this plugin and runtime resolution**

```jsx
import './styles-01.css';
import styles2 from './styles-02.css';

export default function Component({ special }) {
  let textStyle = 'text';
  if (special) textStyle += ' styles2.special';

  return (
    <div styleName="container">
      <h1 styleName="title">Example</div>
      <p styleName={textStyle}>Sample text paragraph.</p>
      <p styleName={textStyle}>
        Sample text paragraph with special style.
      </p>
    </div>
  );
}
```
In the case when the exact style value is not known at the compile time, like in
this example, the plugin will inject necessary code to correctly resolve the
`styleName` at runtime (which is somewhat less performant, but otherwise works
fine).


## Installation

- The core CSS Modules functionality should be enabled and configured elsewhere
  in your React project:
  - example
    [`modules` option of `css-loader`](https://webpack.js.org/loaders/css-loader/#modules).

- Install this plugin as a direct dependency (in edge-cases not allowing for
  a compile-time `styleName` resolution, the plugin falls back to the runtime
  resolution).
  ```
  npm install --save swc-plugin-react-css-modules
  ```

- Install [generic-names](https://www.npmjs.com/package/generic-names) as dependency. Use `generic-names` to provide [getLocalIdent](https://webpack.js.org/loaders/css-loader/#getlocalident) function to css-loader
  ```
  npm install --save generic-names
  ```

- Add the plugin to SWC configuration:
  ```js
  {
    "experimental": {
      "plugins": [
        [
          "swc-plugin-react-css-modules",
          { "generate_scoped_name": "[name]__[local]_[hash:base64:5]" }
        ]
      ]
    }
  }
  ```

- The `generate_scoped_name` option value MUST match `pattern` option of
  `generic-names` to ensure both Webpack and this plugin generate matching class
  names. 

## Configuration

### Plugin Options

- `generate_scoped_name` - **string** - Allows to configure the generated local ident name.
  Must match `generic-names` `pattern` parameter.
  Defaults `[hash:base64]`.
- `hash_prefix` - **string** - Add custom hash prefix to generate more unique classes.
- `css_modules_suffix` - **string** this plugin will consider only those import declarations whose
  src path ends with the given suffix 
  Defaults `.css`.
- `root` - **string** - If the root of the project is not cwd. This option can be used to provide correct value

## Acknowledgements

- Implementation of rust ports of generic-names and loader-utils were done by [swc-plugin-css-modules](https://github.com/VKCOM/swc-plugin-css-modules)