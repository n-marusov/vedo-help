import { SeverityNumber, logs } from '@opentelemetry/api-logs';
import { OTLPLogExporter } from '@opentelemetry/exporter-logs-otlp-http';
import { Resource } from '@opentelemetry/resources';
import {
  BatchLogRecordProcessor,
  ConsoleLogRecordExporter,
  LoggerProvider,
  SimpleLogRecordProcessor,
} from '@opentelemetry/sdk-logs';
import { SemanticResourceAttributes } from '@opentelemetry/semantic-conventions';

const serviceName = 'vedo-frontend';
const serviceVersion = '0.1.0';
const environment = import.meta.env.VITE_ENVIRONMENT || 'development';

const otelCollectorUrl = import.meta.env.VITE_OTEL_COLLECTOR_URL || 'http://localhost:4318/v1/logs';

let loggerProvider: LoggerProvider | null = null;

export function initTelemetry(): void {
  const resource = new Resource({
    [SemanticResourceAttributes.SERVICE_NAME]: serviceName,
    [SemanticResourceAttributes.SERVICE_VERSION]: serviceVersion,
    [SemanticResourceAttributes.DEPLOYMENT_ENVIRONMENT]: environment,
  });

  loggerProvider = new LoggerProvider({ resource });

  // Batch processor for production (OTLP HTTP export)
  const otlpExporter = new OTLPLogExporter({ url: otelCollectorUrl });
  loggerProvider.addLogRecordProcessor(new BatchLogRecordProcessor(otlpExporter));

  // Simple processor for dev mode (console output)
  if (environment === 'development') {
    loggerProvider.addLogRecordProcessor(
      new SimpleLogRecordProcessor(new ConsoleLogRecordExporter()),
    );
  }

  logs.setGlobalLoggerProvider(loggerProvider);
}

export function getLogger() {
  if (!loggerProvider) {
    initTelemetry();
  }
  return logs.getLogger(serviceName);
}

export async function shutdownLogs(): Promise<void> {
  if (loggerProvider) {
    await loggerProvider.shutdown();
    loggerProvider = null;
  }
}

export { SeverityNumber };

export const logger = getLogger();
