/**
 * Comprehensive Logging System for Qualia Client
 * Supports both web (localStorage) and desktop (file system) modes
 */

export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
  CRITICAL = 4,
}

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  levelName: string;
  message: string;
  context?: string;
  data?: any;
  source?: string;
}

class Logger {
  private logs: LogEntry[] = [];
  private maxLogs = 10000; // Maximum logs to keep in memory
  private maxDiskLogs = 100000; // Maximum logs to keep on disk
  private logBuffer: LogEntry[] = [];
  private flushInterval = 5000; // Flush to disk every 5 seconds
  private flushTimer: NodeJS.Timeout | null = null;
  private isDesktop = false;

  constructor() {
    this.isDesktop = typeof window !== 'undefined' && 
                     '__TAURI__' in window && 
                     window.__TAURI__ !== undefined;
    
    this.loadLogsFromStorage();
    this.startFlushTimer();
    
    // Log initialization
    this.info('Logger', 'Logging system initialized', { mode: this.isDesktop ? 'desktop' : 'web' });
  }

  private startFlushTimer() {
    if (typeof window !== 'undefined') {
      this.flushTimer = window.setInterval(() => {
        this.flushToStorage();
      }, this.flushInterval);
    }
  }

  private stopFlushTimer() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
  }

  private async loadLogsFromStorage() {
    try {
      if (this.isDesktop) {
        // Desktop mode: Load from file system
        const { fs } = await import('../lib/tauri-compat');
        const logPath = await this.getLogPath();
        try {
          const content = await fs.readTextFile(logPath);
          const lines = content.split('\n').filter(line => line.trim());
          this.logs = lines.map(line => {
            try {
              return JSON.parse(line);
            } catch {
              return null;
            }
          }).filter(Boolean) as LogEntry[];
        } catch {
          // File doesn't exist yet, that's fine
          this.logs = [];
        }
      } else {
        // Web mode: Load from localStorage
        const stored = localStorage.getItem('qualia_logs');
        if (stored) {
          try {
            const parsed = JSON.parse(stored);
            this.logs = Array.isArray(parsed) ? parsed : [];
          } catch {
            this.logs = [];
          }
        }
      }
      
      // Trim to max logs
      if (this.logs.length > this.maxLogs) {
        this.logs = this.logs.slice(-this.maxLogs);
      }
    } catch (error) {
      console.error('Failed to load logs from storage:', error);
      this.logs = [];
    }
  }

  private async getLogPath(): Promise<string> {
    const { path } = await import('../lib/tauri-compat');
    const appDataDir = await path.appDataDir();
    return await path.join(appDataDir, 'qualia-logs.jsonl');
  }

  private async flushToStorage() {
    if (this.logBuffer.length === 0) return;

    const logsToFlush = [...this.logBuffer];
    this.logBuffer = [];

    try {
      if (this.isDesktop) {
        // Desktop mode: Append to file system
        const { fs } = await import('../lib/tauri-compat');
        const logPath = await this.getLogPath();
        
        // Read existing logs
        let existingLogs: string[] = [];
        try {
          const content = await fs.readTextFile(logPath);
          existingLogs = content.split('\n').filter(line => line.trim());
        } catch {
          // File doesn't exist yet
        }
        
        // Add new logs
        const newLogLines = logsToFlush.map(log => JSON.stringify(log));
        const allLogs = [...existingLogs, ...newLogLines];
        
        // Trim to max disk logs
        if (allLogs.length > this.maxDiskLogs) {
          allLogs.splice(0, allLogs.length - this.maxDiskLogs);
        }
        
        // Write back
        await fs.writeTextFile(logPath, allLogs.join('\n'));
      } else {
        // Web mode: Update localStorage
        const allLogs = [...this.logs, ...logsToFlush];
        
        // Trim to max logs
        if (allLogs.length > this.maxLogs) {
          allLogs.splice(0, allLogs.length - this.maxLogs);
        }
        
        localStorage.setItem('qualia_logs', JSON.stringify(allLogs));
      }
    } catch (error) {
      console.error('Failed to flush logs to storage:', error);
      // Add logs back to buffer for retry
      this.logBuffer.unshift(...logsToFlush);
    }
  }

  private addLog(level: LogLevel, levelName: string, context: string, message: string, data?: any) {
    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      levelName,
      context,
      message,
      data,
      source: this.isDesktop ? 'desktop' : 'web',
    };

    // Add to in-memory logs
    this.logs.push(entry);
    
    // Trim in-memory logs
    if (this.logs.length > this.maxLogs) {
      this.logs.splice(0, this.logs.length - this.maxLogs);
    }
    
    // Add to flush buffer
    this.logBuffer.push(entry);
    
    // Console output for development
    if (process.env.NODE_ENV === 'development') {
      const consoleMethod = level >= LogLevel.ERROR ? 'error' : 
                           level >= LogLevel.WARN ? 'warn' : 'log';
      console[consoleMethod](`[${levelName}] ${context}: ${message}`, data || '');
    }
  }

  debug(context: string, message: string, data?: any) {
    this.addLog(LogLevel.DEBUG, 'DEBUG', context, message, data);
  }

  info(context: string, message: string, data?: any) {
    this.addLog(LogLevel.INFO, 'INFO', context, message, data);
  }

  warn(context: string, message: string, data?: any) {
    this.addLog(LogLevel.WARN, 'WARN', context, message, data);
  }

  error(context: string, message: string, data?: any) {
    this.addLog(LogLevel.ERROR, 'ERROR', context, message, data);
  }

  critical(context: string, message: string, data?: any) {
    this.addLog(LogLevel.CRITICAL, 'CRITICAL', context, message, data);
  }

  getLogs(filter?: {
    level?: LogLevel;
    context?: string;
    startTime?: Date;
    endTime?: Date;
    limit?: number;
  }): LogEntry[] {
    let filtered = [...this.logs];

    if (filter) {
      if (filter.level !== undefined) {
        filtered = filtered.filter(log => log.level >= filter.level!);
      }
      if (filter.context) {
        filtered = filtered.filter(log => 
          log.context?.toLowerCase().includes(filter.context!.toLowerCase())
        );
      }
      if (filter.startTime) {
        filtered = filtered.filter(log => 
          new Date(log.timestamp) >= filter.startTime!
        );
      }
      if (filter.endTime) {
        filtered = filtered.filter(log => 
          new Date(log.timestamp) <= filter.endTime!
        );
      }
      if (filter.limit) {
        filtered = filtered.slice(-filter.limit);
      }
    }

    return filtered;
  }

  getLogsByContext(context: string): LogEntry[] {
    return this.getLogs({ context });
  }

  getLogsByLevel(level: LogLevel): LogEntry[] {
    return this.getLogs({ level });
  }

  getRecentLogs(limit: number = 100): LogEntry[] {
    return this.getLogs({ limit });
  }

  async exportLogs(format: 'json' | 'txt' = 'json'): Promise<string> {
    const logs = this.getLogs();
    
    if (format === 'json') {
      return JSON.stringify(logs, null, 2);
    } else {
      return logs.map(log => 
        `[${log.timestamp}] [${log.levelName}] [${log.context || 'System'}] ${log.message}${
          log.data ? ' ' + JSON.stringify(log.data) : ''
        }`
      ).join('\n');
    }
  }

  async clearLogs(): Promise<void> {
    this.logs = [];
    this.logBuffer = [];
    
    try {
      if (this.isDesktop) {
        const { fs } = await import('../lib/tauri-compat');
        const logPath = await this.getLogPath();
        await fs.writeTextFile(logPath, '');
      } else {
        localStorage.removeItem('qualia_logs');
      }
      
      this.info('Logger', 'Logs cleared');
    } catch (error) {
      console.error('Failed to clear logs:', error);
      this.error('Logger', 'Failed to clear logs', { error: String(error) });
    }
  }

  getLogStats() {
    const logs = this.getLogs();
    const byLevel = {
      debug: logs.filter(l => l.level === LogLevel.DEBUG).length,
      info: logs.filter(l => l.level === LogLevel.INFO).length,
      warn: logs.filter(l => l.level === LogLevel.WARN).length,
      error: logs.filter(l => l.level === LogLevel.ERROR).length,
      critical: logs.filter(l => l.level === LogLevel.CRITICAL).length,
    };
    
    const contexts = [...new Set(logs.map(l => l.context).filter(Boolean))];
    
    return {
      total: logs.length,
      byLevel,
      contexts: contexts.length,
      oldestLog: logs[0]?.timestamp || null,
      newestLog: logs[logs.length - 1]?.timestamp || null,
    };
  }

  destroy() {
    this.stopFlushTimer();
    this.flushToStorage(); // Final flush
  }
}

// Singleton instance
let loggerInstance: Logger | null = null;

export const getLogger = (): Logger => {
  if (!loggerInstance) {
    loggerInstance = new Logger();
  }
  return loggerInstance;
};

export const logger = getLogger();

// Convenience exports
export const debug = (context: string, message: string, data?: any) => logger.debug(context, message, data);
export const info = (context: string, message: string, data?: any) => logger.info(context, message, data);
export const warn = (context: string, message: string, data?: any) => logger.warn(context, message, data);
export const error = (context: string, message: string, data?: any) => logger.error(context, message, data);
export const critical = (context: string, message: string, data?: any) => logger.critical(context, message, data);

export default logger;