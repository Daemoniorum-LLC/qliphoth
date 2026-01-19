// Playground page component - Athame code editor

const defaultCode = `// Welcome to the Sigil Playground!
// Try editing this code and click "Run"

invoke std·io·println;

rite main() {
    // Immutable binding with known evidentiality
    ≔ greeting! = "Hello, Sigil!";

    // Mutable binding
    ≔ vary count! = 0;

    // Loop with native syntax
    ⟳ count < 5 {
        println(format!("{}: {}", count, greeting));
        count += 1;
    }

    // Pattern matching
    ≔ result! = compute(42);
    ⌥ result! {
        ResultI64·Ok(value) => println(format!("Result: {}", value)),
        ResultI64·Err(msg) => println(format!("Error: {}", msg)),
    }
}

rite compute(x: i64) → ResultI64! {
    ⎇ x > 0 {
        ResultI64·Ok(x * 2)
    } ⎉ {
        ResultI64·Err("Input must be positive")
    }
}

ᛈ ResultI64 {
    Ok(i64),
    Err(String),
}`

export function renderPlayground(): string {
  return `
    <div class="playground" data-testid="playground">
      <div class="playground-header" data-testid="playground-header">
        <h1>Sigil Playground</h1>
        <div class="playground-actions">
          <button class="btn btn-primary" data-testid="run-btn">
            ▶ Run
          </button>
          <button class="btn btn-secondary" data-testid="format-btn">
            Format
          </button>
          <button class="btn btn-secondary" data-testid="share-btn">
            Share
          </button>
          <select class="example-select" data-testid="example-select">
            <option value="hello">Hello World</option>
            <option value="fibonacci">Fibonacci</option>
            <option value="counter">Counter Component</option>
            <option value="fetch">Async Fetch</option>
          </select>
        </div>
      </div>

      <div class="playground-content" data-testid="playground-content">
        <div class="editor-panel" data-testid="editor-panel">
          <div class="panel-header">
            <span class="panel-title">Editor</span>
            <span class="panel-info">main.sigil</span>
          </div>
          <div class="athame-editor" data-testid="athame-editor">
            <div class="editor-gutter" data-testid="editor-gutter">
              ${Array.from({ length: 45 }, (_, i) => `<div class="line-number">${i + 1}</div>`).join('')}
            </div>
            <textarea
              class="editor-textarea"
              data-testid="editor-textarea"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            >${defaultCode}</textarea>
          </div>
        </div>

        <div class="output-panel" data-testid="output-panel">
          <div class="panel-tabs" data-testid="output-tabs">
            <button class="panel-tab panel-tab--active" data-testid="output-tab">Output</button>
            <button class="panel-tab" data-testid="wasm-tab">WASM</button>
            <button class="panel-tab" data-testid="ast-tab">AST</button>
          </div>
          <div class="panel-content">
            <div class="output-console" data-testid="output-console">
              <div class="console-line" data-testid="console-line">
                <span class="console-prompt">$</span>
                <span class="console-text">Click "Run" to execute your code</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="playground-footer" data-testid="playground-footer">
        <div class="status-bar">
          <span class="status-item" data-testid="status-line">Ln 1, Col 1</span>
          <span class="status-item" data-testid="status-lang">Sigil</span>
          <span class="status-item" data-testid="status-encoding">UTF-8</span>
        </div>
      </div>
    </div>
  `
}
