import React from 'react';
import { Incremark, ThemeProvider, useIncremark } from '@incremark/react';
import '@incremark/theme/styles.css';

interface ScriptModuleProps {
  content: string;
  onChange?: (content: string) => void;
  readOnly?: boolean;
}

export const ScriptModule: React.FC<ScriptModuleProps> = ({ content, onChange, readOnly = false }) => {
  const incremark = useIncremark();

  // Sync content updates
  React.useEffect(() => {
    if (content !== incremark.markdown) {
      incremark.render(content);
    }
  }, [content, incremark]);

  return (
    <div className="w-full h-full flex flex-col bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-200">
      <div className="flex-1 overflow-hidden relative p-4">
        {readOnly ? (
          <div className="h-full overflow-auto custom-scrollbar">
            <ThemeProvider theme="dark">
              <Incremark incremark={incremark} />
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className="w-full h-full p-4 bg-transparent resize-none focus:outline-none font-mono text-sm border border-zinc-200 dark:border-zinc-800 rounded-lg"
            value={content}
            onChange={(e) => onChange?.(e.target.value)}
            placeholder="Enter script markdown here..."
          />
        )}
      </div>
    </div>
  );
};
