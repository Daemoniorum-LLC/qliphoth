# Contributing to Qliphoth

Thank you for your interest in contributing to Qliphoth! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### Reporting Issues

- Check existing issues to avoid duplicates
- Use a clear, descriptive title
- Include steps to reproduce for bugs
- Specify your environment (OS, Sigil version, browser)

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Ensure tests pass (`npm run test:e2e`)
5. Commit with clear messages
6. Push to your fork
7. Open a Pull Request

### Commit Messages

We follow conventional commits:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions/changes
- `chore:` Maintenance tasks

### Code Style

- Follow existing patterns in the codebase
- Use Sigil's idiomatic conventions
- Include SPDX license headers in new files:
  ```sigil
  // SPDX-License-Identifier: MIT OR Apache-2.0
  // Copyright (c) 2025 Daemoniorum, LLC
  ```

### Testing

- Add tests for new features
- Ensure E2E tests pass before submitting
- Test across multiple browsers when relevant

## Development Setup

1. Clone the repository
2. Install dependencies: `npm install`
3. Start dev server: `npm run dev`
4. Run tests: `npm run test:e2e`

## Project Structure

```
src/
  core/       # Runtime, reconciliation, VDOM
  components/ # Component system
  hooks/      # React-style hooks
  router/     # Client-side routing
  state/      # State management
  platform/   # Cross-platform abstraction
packages/
  qliphoth-sys/    # System bindings
  qliphoth-router/ # Router package
```

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 dual license.

## Questions?

Open an issue or reach out to the maintainers. We're happy to help!
