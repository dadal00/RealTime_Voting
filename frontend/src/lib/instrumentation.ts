import { defaultResource, resourceFromAttributes } from '@opentelemetry/resources'
import { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } from '@opentelemetry/semantic-conventions'
import { WebTracerProvider } from '@opentelemetry/sdk-trace-web'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-proto'
import { BatchSpanProcessor } from '@opentelemetry/sdk-trace-base'

const resource = defaultResource().merge(
  resourceFromAttributes({
    [ATTR_SERVICE_NAME]: 'counter_frontend',
    [ATTR_SERVICE_VERSION]: '0.1.0',
  })
)

const exporter = new OTLPTraceExporter({
  url: 'https://pickone/api/otel/v1/traces',
})
const processor = new BatchSpanProcessor(exporter)

const provider = new WebTracerProvider({
  resource: resource,
  spanProcessors: [processor],
})

provider.register()
