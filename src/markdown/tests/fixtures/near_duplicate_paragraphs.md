# Overview of the analyzer service

The analyzer service is a stateless process that receives document text from the dispatcher and returns metric records computed from that text. It runs as a containerized workload and scales horizontally based on queue depth. Operators manage it through the standard platform tooling.

The analyzer service is a stateless process that receives document text from the dispatcher and returns metric records computed from that content. It runs as a containerized workload and scales horizontally based on queue depth. Operators manage it through the standard platform tooling.

## Configuration

The configuration file lives at a well known path and is read once at process startup. Any changes to the file take effect after a restart of the service. The file follows the standard YAML format and is validated against a schema at load time.

The configuration file lives at a well known path and is read once at process startup. Any changes to the file take effect after a restart of the service. The file follows the standard YAML format and is validated against a schema at load time.

## Deployment

Deployments follow the standard platform flow for containerized services. The image is built during the release pipeline and pushed to the internal registry. Each deploy creates a new rollout and waits for health probes to pass before marking the rollout complete.

Deployments follow the standard platform flow for containerized services. The image is built during the release pipeline and pushed to the internal registry. Each deploy creates a new rollout and waits for health probes to succeed before marking the rollout complete.

## Monitoring

Metrics are emitted from the analyzer process using the standard client library. Dashboards visualize request rates, error rates, and latency distributions. Alerts fire when error rates exceed configured thresholds or when latency percentiles drift outside expected ranges.

Logs are structured JSON and are aggregated through the central logging infrastructure. Traces are emitted through the standard tracing library and allow operators to follow request flow across services.

## Security

Access to the analyzer is controlled through the standard authentication layer. Requests must carry a valid authentication token or they are rejected with an HTTP 401 response. Rate limits apply per tenant and are enforced at the ingress layer rather than inside the analyzer itself.
