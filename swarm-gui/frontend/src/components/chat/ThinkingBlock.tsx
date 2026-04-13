import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { ChevronDown, ChevronRight, Brain } from "lucide-react";
import { clsx } from "clsx";

interface ThinkingBlockProps {
  content: string;
  defaultExpanded?: boolean;
}

export default function ThinkingBlock({ content, defaultExpanded = false }: ThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  return (
    <div className="thinking-block mb-3">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={clsx(
          "flex items-center gap-2 p-1.5 text-xs font-medium transition-colors rounded hover:bg-black/5 dark:hover:bg-white/5",
          isExpanded ? "text-primary-foreground/70" : "text-muted-foreground"
        )}
      >
        {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <Brain size={14} className="text-primary/60" />
        <span className="italic">Thinking...</span>
      </button>
      
      {isExpanded && (
        <div className="mt-1 ml-6 p-3 border-l-2 border-primary/20 bg-muted/30 rounded-r text-sm text-foreground/80 italic">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {content}
          </ReactMarkdown>
        </div>
      )}
    </div>
  );
}
