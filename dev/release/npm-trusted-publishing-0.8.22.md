# 0.8.22 npm trusted-publishing bootstrap

Run this once, manually, before a real 0.8.22 tag is pushed. It grants no
long-lived npm token to GitHub.

1. In npm, create or claim these public packages: `fathomdb`,
   `fathomdb-darwin-x64`, `fathomdb-darwin-arm64`, and
   `fathomdb-native-win32-x64-msvc`. The Windows x64 MSVC package is the
   explicit naming exception; the macOS packages remain unscoped
   `fathomdb-<triple>` names.
2. For each package, configure GitHub Actions trusted publishing for repository
   `coreyt/fathomdb` with workflow filename `release.yml`. Leave the npm
   environment unset: no npm publish job uses a GitHub Actions environment.
   Allow npm publish for that trusted publisher.
3. Verify the npm package owner permits provenance and that the GitHub workflow
   has `id-token: write` on every platform-publish job and `publish-npm`.
4. Run the release workflow as a dry run from the immutable candidate commit;
   confirm every new platform job stages exactly one matching `.node` file.
5. Before the real run, record the npm UI configuration and obtain the normal
   explicit HITL publish authorization. Do not add `NPM_TOKEN` or another
   long-lived npm credential as a repository secret.

The real run publishes every platform package and the thin `fathomdb` package
under `next`. Only `fathomdb@<version>` is promoted to `latest`, and only after
all five actual-runner registry smokes and co-tagging succeed.
