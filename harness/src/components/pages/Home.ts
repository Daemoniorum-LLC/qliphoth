// Home page component

export function renderHome(): string {
  return `
    <div class="home" data-testid="home">
    <section class="hero" data-testid="hero">
      <div class="hero-content">
        <h1 class="hero-title" data-testid="hero-title">
          Build the Future with
          <span class="hero-highlight">Sigil</span>
        </h1>
        <p class="hero-tagline" data-testid="hero-tagline">
          A polysynthetic systems programming language with evidentiality types,
          morpheme composition, and Rust-level performance.
        </p>
        <div class="hero-actions" data-testid="hero-actions">
          <a href="/docs" class="btn btn-primary" data-testid="cta-docs">
            Documentation
          </a>
          <a href="/playground" class="btn btn-secondary" data-testid="cta-playground">
            Try Playground
          </a>
        </div>
      </div>
    </section>

    <section class="features" data-testid="features">
      <h2 class="section-title">Why Sigil?</h2>
      <div class="feature-grid">
        <div class="feature-card" data-testid="feature-evidentiality">
          <div class="feature-icon">🔒</div>
          <h3 class="feature-title">Evidentiality Types</h3>
          <p class="feature-description">
            Track data provenance at the type level. Know where your data comes from
            and how it was transformed.
          </p>
        </div>
        <div class="feature-card" data-testid="feature-performance">
          <div class="feature-icon">⚡</div>
          <h3 class="feature-title">Zero-Cost Abstractions</h3>
          <p class="feature-description">
            Express high-level concepts without runtime overhead. Compile to
            efficient native code via LLVM.
          </p>
        </div>
        <div class="feature-card" data-testid="feature-morphemes">
          <div class="feature-icon">🧩</div>
          <h3 class="feature-title">Morpheme Composition</h3>
          <p class="feature-description">
            Build complex expressions from simple primitives using Unicode
            operators and pipe transformations.
          </p>
        </div>
        <div class="feature-card" data-testid="feature-wasm">
          <div class="feature-icon">🌐</div>
          <h3 class="feature-title">WASM Ready</h3>
          <p class="feature-description">
            Compile to WebAssembly for web applications. Use Qliphoth for
            React-inspired component development.
          </p>
        </div>
      </div>
    </section>

    <section class="products" data-testid="products">
      <h2 class="section-title">Ecosystem</h2>
      <div class="product-grid">
        <a href="/docs/sigil" class="product-card" data-testid="product-sigil">
          <div class="product-icon">☉</div>
          <h3 class="product-name">Sigil</h3>
          <p class="product-tagline">Systems programming language</p>
        </a>
        <a href="/docs/qliphoth" class="product-card" data-testid="product-qliphoth">
          <div class="product-icon">🌐</div>
          <h3 class="product-name">Qliphoth</h3>
          <p class="product-tagline">Web framework</p>
        </a>
        <a href="/docs/leviathan" class="product-card" data-testid="product-leviathan">
          <div class="product-icon">🖥️</div>
          <h3 class="product-name">Leviathan</h3>
          <p class="product-tagline">Backend framework</p>
        </a>
        <a href="/docs/nyx" class="product-card" data-testid="product-nyx">
          <div class="product-icon">🧠</div>
          <h3 class="product-name">Nyx</h3>
          <p class="product-tagline">AI agent framework</p>
        </a>
      </div>
    </section>

    <section class="code-preview" data-testid="code-preview">
      <h2 class="section-title">See It In Action</h2>
      <div class="code-block" data-testid="code-block">
        <div class="code-header">
          <span class="code-filename">hello.sigil</span>
          <button class="code-copy" data-testid="copy-code">Copy</button>
        </div>
        <pre class="code-content"><code>// Native Sigil syntax
invoke std·io·println;

☉ rite main() {
    ≔ greeting! = "Hello, Sigil!";
    ≔ count! = 42;

    // Evidentiality tracking
    ≔ data~ = fetch_api("/users")|await;

    // Pattern matching
    ⌥ data~ {
        ResultUsers·Ok(users) => {
            ∀ user ∈ users {
                println(format!("User: {}", user.name));
            }
        }
        ResultUsers·Err(e) => {
            println(format!("Error: {}", e));
        }
    }
}</code></pre>
      </div>
    </section>
    </div>
  `
}
