# Parked workflows

`deploy.yml` triggers a Render redeploy via the `RENDER_DEPLOY_HOOK`
secret. It is intentionally disabled (kept out of `.github/workflows/`)
so pushes do not auto-deploy.

To re-enable: move the file back into `.github/workflows/`.
