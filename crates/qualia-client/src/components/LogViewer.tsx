import { useState, useEffect } from 'react';
import { Download, Trash2, Filter, Search, RefreshCw, AlertCircle, CheckCircle, X, FileText, Activity } from 'lucide-react';
import { logger, LogLevel, LogEntry } from '../lib/logger';

interface LogViewerProps {
  onClose?: () => void;
  embedded?: boolean;
}

export default function LogViewer({ onClose, embedded = false }: LogViewerProps) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filteredLogs, setFilteredLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState({
    level: undefined as LogLevel | undefined,
    context: '',
    search: '',
  });
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [stats, setStats] = useState(logger.getLogStats());

  useEffect(() => {
    loadLogs();
    if (autoRefresh) {
      const interval = setInterval(() => {
        loadLogs();
        setStats(logger.getLogStats());
      }, 2000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh, filter]);

  const loadLogs = () => {
    const allLogs = logger.getLogs({
      level: filter.level,
      context: filter.context || undefined,
      limit: 1000,
    });
    
    let filtered = allLogs;
    if (filter.search) {
      const searchLower = filter.search.toLowerCase();
      filtered = filtered.filter(log =>
        log.message.toLowerCase().includes(searchLower) ||
        log.context?.toLowerCase().includes(searchLower) ||
        (log.data && JSON.stringify(log.data).toLowerCase().includes(searchLower))
      );
    }
    
    setLogs(allLogs);
    setFilteredLogs(filtered);
  };

  const handleExport = async (format: 'json' | 'txt') => {
    try {
      const content = await logger.exportLogs(format);
      const blob = new Blob([content], { 
        type: format === 'json' ? 'application/json' : 'text/plain' 
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `qualia-logs-${new Date().toISOString().split('T')[0]}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Failed to export logs:', error);
    }
  };

  const handleClearLogs = async () => {
    if (confirm('Are you sure you want to clear all logs? This action cannot be undone.')) {
      await logger.clearLogs();
      loadLogs();
      setStats(logger.getLogStats());
    }
  };

  const getLevelColor = (level: LogLevel) => {
    switch (level) {
      case LogLevel.DEBUG: return 'text-gray-400';
      case LogLevel.INFO: return 'text-blue-400';
      case LogLevel.WARN: return 'text-yellow-400';
      case LogLevel.ERROR: return 'text-red-400';
      case LogLevel.CRITICAL: return 'text-red-600 font-bold';
      default: return 'text-gray-400';
    }
  };

  const getLevelIcon = (level: LogLevel) => {
    switch (level) {
      case LogLevel.DEBUG: return <Activity className="w-4 h-4" />;
      case LogLevel.INFO: return <CheckCircle className="w-4 h-4" />;
      case LogLevel.WARN: return <AlertCircle className="w-4 h-4" />;
      case LogLevel.ERROR: return <X className="w-4 h-4" />;
      case LogLevel.CRITICAL: return <AlertCircle className="w-4 h-4" />;
      default: return <FileText className="w-4 h-4" />;
    }
  };

  return (
    <div className={`flex flex-col gap-6 ${embedded ? '' : 'h-full'}`}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-white">System Logs</h2>
          <div className="flex gap-4 mt-2 text-sm text-gray-400">
            <span>Total: <strong className="text-white">{stats.total}</strong></span>
            <span>Errors: <strong className="text-red-400">{stats.byLevel.error + stats.byLevel.critical}</strong></span>
            <span>Warnings: <strong className="text-yellow-400">{stats.byLevel.warn}</strong></span>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`p-2 rounded-lg border transition-all ${
              autoRefresh 
                ? 'bg-[#00ff88]/10 text-[#00ff88] border-[#00ff88]/30' 
                : 'bg-white/5 text-gray-400 border-white/10'
            }`}
            title="Auto-refresh"
          >
            <RefreshCw className={`w-4 h-4 ${autoRefresh ? 'animate-spin' : ''}`} />
          </button>
          <button
            onClick={() => handleExport('json')}
            className="p-2 rounded-lg bg-white/5 text-gray-400 border border-white/10 hover:bg-white/10 transition-all"
            title="Export as JSON"
          >
            <Download className="w-4 h-4" />
          </button>
          <button
            onClick={() => handleExport('txt')}
            className="p-2 rounded-lg bg-white/5 text-gray-400 border border-white/10 hover:bg-white/10 transition-all"
            title="Export as TXT"
          >
            <FileText className="w-4 h-4" />
          </button>
          <button
            onClick={handleClearLogs}
            className="p-2 rounded-lg bg-red-500/10 text-red-400 border border-red-500/30 hover:bg-red-500/20 transition-all"
            title="Clear logs"
          >
            <Trash2 className="w-4 h-4" />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-2 rounded-lg bg-white/5 text-gray-400 border border-white/10 hover:bg-white/10 transition-all"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Filters */}
      <div className="glass-panel p-4">
        <div className="flex gap-4 items-center">
          <div className="flex items-center gap-2 flex-1">
            <Search className="w-4 h-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search logs..."
              value={filter.search}
              onChange={(e) => setFilter({ ...filter, search: e.target.value })}
              className="flex-1 bg-black/50 border border-white/20 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-[#00f0ff] text-sm"
            />
          </div>
          
          <select
            value={filter.level ?? ''}
            onChange={(e) => setFilter({ ...filter, level: e.target.value ? Number(e.target.value) as LogLevel : undefined })}
            className="bg-black/50 border border-white/20 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-[#00f0ff] text-sm"
          >
            <option value="">All Levels</option>
            <option value={LogLevel.DEBUG}>Debug</option>
            <option value={LogLevel.INFO}>Info</option>
            <option value={LogLevel.WARN}>Warning</option>
            <option value={LogLevel.ERROR}>Error</option>
            <option value={LogLevel.CRITICAL}>Critical</option>
          </select>
          
          <input
            type="text"
            placeholder="Filter by context..."
            value={filter.context}
            onChange={(e) => setFilter({ ...filter, context: e.target.value })}
            className="bg-black/50 border border-white/20 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-[#00f0ff] text-sm w-48"
          />
          
          <button
            onClick={loadLogs}
            className="p-2 rounded-lg bg-[#00f0ff]/10 text-[#00f0ff] border border-[#00f0ff]/30 hover:bg-[#00f0ff]/20 transition-all"
          >
            <Filter className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Log entries */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="flex-1 overflow-y-auto glass-panel">
          {filteredLogs.length === 0 ? (
            <div className="flex items-center justify-center h-full text-gray-500">
              No logs found matching the current filters
            </div>
          ) : (
            <div className="font-mono text-sm">
              {filteredLogs.map((log, index) => (
                <div
                  key={index}
                  className={`border-b border-white/5 p-3 hover:bg-white/5 transition-all ${
                    log.level >= LogLevel.ERROR ? 'bg-red-500/5' : ''
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <div className={`mt-0.5 ${getLevelColor(log.level)}`}>
                      {getLevelIcon(log.level)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className={`text-xs font-semibold ${getLevelColor(log.level)}`}>
                          {log.levelName}
                        </span>
                        <span className="text-xs text-gray-500">
                          {new Date(log.timestamp).toLocaleString()}
                        </span>
                        {log.context && (
                          <span className="text-xs text-[#00f0ff] bg-[#00f0ff]/10 px-2 py-0.5 rounded">
                            {log.context}
                          </span>
                        )}
                      </div>
                      <div className="text-gray-300 break-words">{log.message}</div>
                      {log.data && (
                        <details className="mt-2">
                          <summary className="text-xs text-gray-500 cursor-pointer hover:text-gray-300">
                            View data
                          </summary>
                          <pre className="mt-2 text-xs text-gray-400 bg-black/30 p-2 rounded overflow-x-auto">
                            {JSON.stringify(log.data, null, 2)}
                          </pre>
                        </details>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}