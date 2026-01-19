// Doc page component - displays a single documentation page

const mockDocs: Record<string, { title: string; content: string }> = {
  sigil: {
    title: 'Sigil Language',
    content: `
      <p class="lead">
        Sigil is a polysynthetic systems programming language designed to achieve
        Rust-level performance while offering superior expressiveness through
        morpheme composition and evidentiality tracking.
      </p>

      <h2 id="features">Key Features</h2>
      <ul>
        <li><strong>Evidentiality Types</strong> - Track data provenance at the type level</li>
        <li><strong>Morpheme Composition</strong> - Build complex expressions from simple primitives</li>
        <li><strong>Zero-Cost Abstractions</strong> - No runtime overhead for high-level constructs</li>
        <li><strong>LLVM Backend</strong> - Compile to efficient native code</li>
        <li><strong>WASM Support</strong> - Run in browsers and edge environments</li>
      </ul>

      <h2 id="quick-example">Quick Example</h2>
      <div class="code-block">
        <pre><code>// Hello World in Sigil
invoke std·io·println;

rite main() {
    ≔ message! = "Hello, Sigil!";
    println(message!);
}</code></pre>
      </div>

      <h2 id="installation">Installation</h2>
      <div class="code-block">
        <pre><code># Install via curl
curl -fsSL https://sigil-lang.org/install | sh

# Or with cargo (if available)
cargo install sigil-lang</code></pre>
      </div>

      <h2 id="next-steps">Next Steps</h2>
      <ul>
        <li><a href="/docs/sigil/getting-started">Getting Started Guide</a></li>
        <li><a href="/docs/sigil/spec/types">Type System Reference</a></li>
        <li><a href="/playground">Try in Playground</a></li>
      </ul>
    `,
  },
  qliphoth: {
    title: 'Qliphoth - Sigil Web Framework',
    content: `
      <p class="lead">
        Qliphoth is a React-inspired web framework for building interactive
        applications with Sigil, compiled to WebAssembly.
      </p>

      <h2 id="features">Features</h2>
      <ul>
        <li><strong>Component Model</strong> - Functional and stateful components</li>
        <li><strong>Signals</strong> - Fine-grained reactivity system</li>
        <li><strong>Virtual DOM</strong> - Efficient UI updates</li>
        <li><strong>Hooks</strong> - useState, useEffect, useMemo, and more</li>
        <li><strong>Router</strong> - Type-safe client-side routing</li>
      </ul>

      <h2 id="example">Example Component</h2>
      <div class="code-block">
        <pre><code>invoke qliphoth·prelude·*;

component Counter {
    state count: i64! = 0

    rite render(this) → VNode {
        div {
            h1 { "Count: {this.count}" }
            button[onclick: || this.count += 1] { "+" }
            button[onclick: || this.count -= 1] { "-" }
        }
    }
}</code></pre>
      </div>
    `,
  },
}

export function renderDocPage(params: Record<string, string>): string {
  const projectId = params.project || 'sigil'
  const doc = mockDocs[projectId] || mockDocs.sigil

  return `
    <article class="markdown-content" data-testid="doc-page">
      <header class="doc-header">
        <nav class="breadcrumbs" data-testid="breadcrumbs">
          <a href="/docs">Docs</a>
          <span class="separator">/</span>
          <span class="current">${doc.title}</span>
        </nav>
        <h1 data-testid="doc-title">${doc.title}</h1>
        <div class="doc-meta" data-testid="doc-meta">
          <span class="reading-time">5 min read</span>
          <span class="last-updated">Updated Jan 2025</span>
        </div>
      </header>

      <div class="doc-body" data-testid="doc-body">
        ${doc.content}
      </div>

      <footer class="doc-footer" data-testid="doc-footer">
        <nav class="page-nav" data-testid="page-nav">
          <a href="/docs" class="page-nav-link page-nav-link--prev" data-testid="prev-link">
            <span class="page-nav-direction">Previous</span>
            <span class="page-nav-title">Documentation</span>
          </a>
          <a href="/docs/${projectId}/getting-started" class="page-nav-link page-nav-link--next" data-testid="next-link">
            <span class="page-nav-direction">Next</span>
            <span class="page-nav-title">Getting Started</span>
          </a>
        </nav>
      </footer>
    </article>

    <aside class="toc-sidebar" data-testid="toc-sidebar">
      <h4>On this page</h4>
      <nav class="toc-nav" data-testid="toc-nav">
        <a href="#features" class="toc-link">Features</a>
        <a href="#quick-example" class="toc-link">Quick Example</a>
        <a href="#installation" class="toc-link">Installation</a>
        <a href="#next-steps" class="toc-link">Next Steps</a>
      </nav>
    </aside>
  `
}
