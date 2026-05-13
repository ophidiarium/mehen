# Draft runbook

This runbook is a work in progress. Many sections are TODO and should be
filled out before production use.

## Preflight

TODO: describe preflight checks. We need to cover at least the following
items before deploy:

- TODO: verify build artifact exists
- FIXME: add link to the build pipeline
- TBD: owner of the preflight step

See [the CI dashboard](TBD) for the latest run status. Related docs:
[incident response](TODO) and [rollback flow](FIXME).

## Deploy

TODO: write the deploy procedure. The `deploy.sh` script is a placeholder
and will be rewritten before this runbook is finalised. XXX: the current
script does not handle the `staging` environment correctly.

Sample placeholder: lorem ipsum dolor sit amet, consectetur adipiscing
elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

## Health checks

FIXME: document the health check endpoints. Owner: TBD. Target completion
date: TBD.

## Rollback

TODO: describe the rollback path. The previous runbook had a diagram
here but it has been removed as a placeholder.

See also [historical rollback notes](TBD) and [postmortems](TODO).

## Troubleshooting

TODO: fill in common troubleshooting steps. The placeholder section below
captures a few ideas but none of them have been verified.

### Slow response times

FIXME: add specifics. This section is a placeholder.

### High error rates

TODO: document the error budget and the alert thresholds.

## References

- TODO: link to the deploy pipeline
- FIXME: link to the monitoring dashboard
- TBD: link to the security review
- [placeholder entry](TBD)
- [another placeholder]()
