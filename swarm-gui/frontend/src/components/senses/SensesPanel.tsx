import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SymbolResult {
  name: String;
  file: String;
  kind: String;
}

export default function SensesPanel() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SymbolResult[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;

    setLoading(true);
    try {
      const res = await invoke<SymbolResult[]>('senses_search', { query });
      setResults(res);
    } catch (err) {
      console.error('Senses search failed:', err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#0a0a0a] text-gray-300">
      <div className="p-4 border-b border-[#1f1f1f]">
        <h2 className="text-xs font-bold uppercase tracking-widest text-[#00e5ff] mb-4">
          Deep Swarm Senses
        </h2>
        
        <form onSubmit={handleSearch} className="flex flex-col gap-2">
          <div className="relative">
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search functions, classes, structs..."
              className="w-full bg-[#161616] border border-[#2a2a2a] rounded-md py-2 px-3 text-sm focus:outline-none focus:border-[#00e5ff] transition-all"
            />
            {loading && (
              <div className="absolute right-3 top-2.5 animate-spin">
                ⏳
              </div>
            )}
          </div>
          <button 
            type="submit"
            className="w-full py-2 rounded-md bg-[#00e5ff1a] text-[#00e5ff] text-xs font-bold border border-[#00e5ff33] hover:bg-[#00e5ff33] transition-all"
          >
            EXECUTE AST SEARCH
          </button>
        </form>
      </div>

      <div className="flex-1 overflow-auto p-2 scrollbar-hide">
        {results.length === 0 && !loading ? (
          <div className="h-full flex flex-col items-center justify-center opacity-30 text-center px-6">
            <span style={{ fontSize: 40 }} className="mb-4">🧠</span>
            <p className="text-xs">Search for code symbols to see their location and relationships in the graph.</p>
          </div>
        ) : (
          <div className="flex flex-col gap-1">
            {results.map((res, i) => (
              <div 
                key={i} 
                className="group p-3 bg-[#111] border border-transparent hover:border-[#222] hover:bg-[#161616] rounded-md transition-all cursor-pointer"
              >
                <div className="flex justify-between items-start mb-1">
                  <span className="text-sm font-medium text-white group-hover:text-[#00e5ff] transition-colors line-clamp-1">
                    {res.name}
                  </span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#1f1f1f] text-gray-500 font-mono">
                    {res.kind}
                  </span>
                </div>
                <div className="text-[10px] text-gray-500 line-clamp-1 italic">
                  {res.file.split(/[\\/]/).pop()}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
