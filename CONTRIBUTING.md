# Contributing to BUNKER MINER

We're excited that you're interested in contributing to BUNKER MINER! This document outlines the process for contributing to the project and our expectations for contributors.

## 🤝 How to Contribute

### Reporting Issues

If you encounter a bug or have a feature request:

1. **Search existing issues** to avoid duplicates
2. **Use the appropriate issue template** when creating new issues
3. **Provide detailed information** including:
   - Operating system and version
   - Hardware configuration
   - Steps to reproduce (for bugs)
   - Expected vs actual behavior
   - Relevant log files or error messages

### Contributing Code

1. **Fork the repository** to your GitHub account
2. **Create a feature branch** from `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following our coding standards
4. **Test your changes** thoroughly
5. **Commit your changes** with descriptive messages
6. **Push to your fork** and create a pull request

### Pull Request Process

1. **Target the `develop` branch** for all pull requests
2. **Update documentation** if your changes affect user-facing functionality
3. **Add or update tests** to maintain code coverage above 80%
4. **Ensure all tests pass** and pre-commit hooks succeed
5. **Request review** from maintainers
6. **Address feedback** promptly and professionally

## 📋 Development Guidelines

### Code Standards

#### Rust Code
- Follow **Rust official style guidelines** (rustfmt)
- Use **meaningful variable and function names**
- Add **comprehensive documentation** for public APIs
- Include **unit tests** for all new functionality
- Handle **errors gracefully** with proper error types

#### C++ Code (Client)
- Follow **Google C++ Style Guide**
- Use **modern C++ features** (C++17 minimum)
- Apply **RAII principles** for resource management
- Include **Qt-specific best practices**

#### Documentation
- Use **clear, concise language**
- Include **code examples** for complex features
- Keep **README and docs up to date**
- Follow **Markdown formatting standards**

### Security Requirements

All contributions must meet our security standards:

- **No hardcoded secrets** or credentials
- **Input validation** for all user inputs
- **Secure communication** protocols only
- **Principle of least privilege** for system access
- **Security review** for all crypto-related code

### Testing Requirements

- **Unit tests** for all new functions/methods
- **Integration tests** for API endpoints
- **Security tests** for authentication/authorization
- **Performance tests** for critical paths
- **Documentation tests** for code examples

### Commit Message Format

Use conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(daemon): add automatic profit switching
fix(client): resolve connection timeout issues
docs(readme): update installation instructions
```

## 🛠️ Development Setup

### Prerequisites

Ensure you have the development environment set up according to [`docs/DEVELOPMENT_ENVIRONMENT.md`](docs/DEVELOPMENT_ENVIRONMENT.md).

### Local Development

1. **Clone your fork**:
   ```bash
   git clone https://github.com/your-username/bunker-miner.git
   cd bunker-miner
   ```

2. **Install pre-commit hooks**:
   ```bash
   ./tools/setup-pre-commit.sh
   ```

3. **Start development environment**:
   ```bash
   docker-compose -f docker-compose.dev.yml up -d
   ```

4. **Run tests**:
   ```bash
   cargo test
   ```

### Code Review Process

All code changes require review:

1. **Automated checks** must pass (CI/CD pipeline)
2. **At least one approval** from a core maintainer
3. **Security review** for sensitive changes
4. **Performance review** for critical path changes

## 🎯 Areas for Contribution

We welcome contributions in these areas:

### High Priority
- **Mining software integrations** (new miners/algorithms)
- **Hardware support** (new GPU models/drivers)
- **Performance optimizations** (CPU/memory efficiency)
- **Security improvements** (vulnerability fixes)
- **Documentation** (user guides, API docs)

### Medium Priority
- **User interface improvements** (desktop/web)
- **Testing enhancements** (coverage, automation)
- **Monitoring and logging** (observability)
- **Configuration management** (user experience)

### Low Priority
- **Code refactoring** (technical debt)
- **Build system improvements** (CI/CD pipeline)
- **Development tooling** (scripts, automation)

## 🏷️ Issue Labels

We use labels to categorize issues:

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation needs
- `security`: Security-related issues
- `performance`: Performance optimization
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `priority/high`: Critical issues
- `component/daemon`: Daemon-related
- `component/client`: Client-related
- `component/pool`: Pool-related

## 🎉 Recognition

Contributors are recognized in:

- **README.md contributors section**
- **Release notes** for significant contributions
- **Hall of Fame** for major contributors
- **Discord contributor role** for active members

## 📝 License

By contributing to BUNKER MINER, you agree that your contributions will be licensed under the MIT License.

## 💬 Communication

- **Discord**: Join our [community Discord](https://discord.gg/bunker-miner)
- **GitHub Discussions**: For design discussions and questions
- **Email**: For sensitive security issues: security@bunkercorpo.com

## ❓ Questions?

If you have questions about contributing:

1. Check existing **GitHub Discussions**
2. Ask in our **Discord community**
3. Contact maintainers directly for sensitive issues

Thank you for contributing to BUNKER MINER! 🚀