import { useEffect, useRef, useState } from "react";
import { Maximize2, Minimize2, ExternalLink } from "lucide-react";

interface VisualBlockProps {
  html: string;
  mimeType: string;
}

export default function VisualBlock({ html, mimeType }: VisualBlockProps) {
  const [isFullscreen, setIsFullscreen] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  // Auto-resize functionality
  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe) return;

    const handleMessage = (event: MessageEvent) => {
      if (event.data.type === 'resize' && event.data.height) {
        iframe.style.height = `${event.data.height}px`;
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  // Inject resize script into the HTML
  const enhancedHtml = `
    ${html}
    <script>
      function sendHeight() {
        const height = document.documentElement.scrollHeight;
        window.parent.postMessage({ type: 'resize', height: height }, '*');
      }
      window.addEventListener('load', sendHeight);
      window.addEventListener('resize', sendHeight);
      new ResizeObserver(sendHeight).observe(document.body);
    </script>
    <style>
      body { margin: 0; padding: 12px; font-family: sans-serif; overflow: hidden; }
      ::-webkit-scrollbar { width: 4px; height: 4px; }
      ::-webkit-scrollbar-thumb { background: #8884; border-radius: 4px; }
    </style>
  `;

  return (
    <div className={`visual-block my-4 border rounded-lg overflow-hidden bg-white dark:bg-zinc-900 shadow-sm ${isFullscreen ? 'fixed inset-4 z-50' : 'relative'}`}>
      <div className="flex items-center justify-between px-3 py-1.5 bg-muted/20 border-b text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
        <span>Visual Component ({mimeType})</span>
        <div className="flex items-center gap-2">
          <button 
            onClick={() => setIsFullscreen(!isFullscreen)}
            className="hover:text-foreground p-1 transition-colors"
            title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
          >
            {isFullscreen ? <Minimize2 size={12} /> : <Maximize2 size={12} />}
          </button>
          <a
            href={`data:text/html;charset=utf-8,${encodeURIComponent(html)}`}
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-foreground p-1 transition-colors"
            title="Open in new window"
          >
            <ExternalLink size={12} />
          </a>
        </div>
      </div>
      <iframe
        ref={iframeRef}
        srcDoc={enhancedHtml}
        className="w-full transition-[height] duration-200 ease-in-out"
        style={{ height: '300px', border: 'none' }}
        sandbox="allow-scripts allow-forms allow-same-origin"
      />
    </div>
  );
}
