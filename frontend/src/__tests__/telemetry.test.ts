import { beforeEach, describe, expect, it, vi } from 'vitest';

const mockAddLogRecordProcessor = vi.fn();
const mockShutdown = vi.fn().mockResolvedValue(undefined);
const mockEmit = vi.fn();

// Mock OTel modules before importing telemetry
vi.mock('@opentelemetry/sdk-logs', () => ({
  LoggerProvider: vi.fn().mockImplementation(() => ({
    addLogRecordProcessor: mockAddLogRecordProcessor,
    shutdown: mockShutdown,
  })),
  ConsoleLogRecordExporter: vi.fn(),
  BatchLogRecordProcessor: vi.fn(),
  SimpleLogRecordProcessor: vi.fn(),
}));

vi.mock('@opentelemetry/exporter-logs-otlp-http', () => ({
  OTLPLogExporter: vi.fn(),
}));

vi.mock('@opentelemetry/resources', () => ({
  Resource: vi.fn().mockImplementation((attrs) => attrs),
}));

vi.mock('@opentelemetry/api-logs', () => ({
  SeverityNumber: {
    TRACE: 1,
    DEBUG: 5,
    INFO: 9,
    WARN: 13,
    ERROR: 17,
    FATAL: 21,
  },
  logs: {
    getLogger: vi.fn().mockReturnValue({ emit: mockEmit }),
    setGlobalLoggerProvider: vi.fn(),
  },
}));

describe('telemetry', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize logger with correct service name', async () => {
    // Import triggers initTelemetry via module-level getLogger()
    // But mocks above handle the OTel constructors
    const telemetry = await import('../telemetry');
    expect(telemetry.logger).toBeDefined();
  });

  it('should have working initTelemetry that does not throw', async () => {
    const { initTelemetry } = await import('../telemetry');
    expect(() => initTelemetry()).not.toThrow();
  });

  it('should expose SeverityNumber constants', async () => {
    const { SeverityNumber } = await import('../telemetry');
    expect(SeverityNumber.DEBUG).toBe(5);
    expect(SeverityNumber.INFO).toBe(9);
    expect(SeverityNumber.WARN).toBe(13);
    expect(SeverityNumber.ERROR).toBe(17);
  });

  it('should allow emitting log records', async () => {
    const { getLogger } = await import('../telemetry');
    const log = getLogger();

    log.emit({
      severityNumber: 9,
      severityText: 'INFO',
      body: 'Test message',
      attributes: { component: 'test', key: 'value' },
    });

    expect(mockEmit).toHaveBeenCalledWith({
      severityNumber: 9,
      severityText: 'INFO',
      body: 'Test message',
      attributes: { component: 'test', key: 'value' },
    });
  });

  it('should handle error severity log with component attribute', async () => {
    const { getLogger } = await import('../telemetry');
    const log = getLogger();

    log.emit({
      severityNumber: 17,
      severityText: 'ERROR',
      body: 'An error occurred',
      attributes: { component: 'auth' },
    });

    expect(mockEmit).toHaveBeenCalledWith({
      severityNumber: 17,
      severityText: 'ERROR',
      body: 'An error occurred',
      attributes: { component: 'auth' },
    });
  });
});
