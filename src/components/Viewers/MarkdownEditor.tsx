import React, { useState, useEffect, useRef } from 'react';
import { marked } from 'marked';
import { invoke } from '@tauri-apps/api/core';
import { Save, CheckCircle2, Loader2 } from 'lucide-react';

interface MarkdownEditorProps {
  hash?: string;
  initialContent?: string;
  onSave?: (newContent: string) => void;
  onClose?: () => void;
}

export const MarkdownEditor: React.FC<MarkdownEditorProps> = ({
  hash,
  initialContent = '# Untitled Note\n\nStart typing cooperative class notes here...',
  onSave,
  onClose,
}) => {
  const [content, setContent] = useState<string>(initialContent);
  const [saving, setSaving] = useState<boolean>(false);
  const [saved, setSaved] = useState<boolean>(true);
  const [loading, setLoading] = useState<boolean>(false);

  // Load content if hash provided
  useEffect(() => {
    if (!hash) return;
    setLoading(true);
    invoke<number[]>('read_file_bytes', { hash })
      .then((bytes) => {
        if (bytes && bytes.length > 0) {
          const text = new TextDecoder().decode(new Uint8Array(bytes));
          setContent(text);
        }
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to load markdown file:', err);
        setLoading(false);
      });
  }, [hash]);

  // Debounced auto-save
  const timerRef = useRef<any>(null);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const text = e.target.value;
    setContent(text);
    setSaved(false);

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(async () => {
      setSaving(true);
      if (onSave) {
        onSave(text);
      }
      setSaving(false);
      setSaved(true);
    }, 1000);
  };

  const getHtml = () => {
    try {
      return marked.parse(content, { async: false }) as string;
    } catch {
      return '<p>Error rendering markdown</p>';
    }
  };

  return (
    <div className="flex flex-col h-full bg-white">
      {/* Top Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-gray-700">Markdown Editor</span>
          {loading ? (
            <span className="flex items-center text-xs text-gray-500 gap-1">
              <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading...
            </span>
          ) : saving ? (
            <span className="flex items-center text-xs text-blue-600 gap-1">
              <Save className="w-3.5 h-3.5 animate-pulse" /> Saving...
            </span>
          ) : saved ? (
            <span className="flex items-center text-xs text-green-600 gap-1">
              <CheckCircle2 className="w-3.5 h-3.5" /> Saved
            </span>
          ) : (
            <span className="text-xs text-amber-600">Unsaved changes</span>
          )}
        </div>

        {onClose && (
          <button
            onClick={onClose}
            className="px-3 py-1 text-xs bg-gray-200 hover:bg-gray-300 rounded font-medium"
          >
            Close
          </button>
        )}
      </div>

      {/* Split Pane */}
      <div className="flex-1 flex overflow-hidden">
        {/* Editor Area */}
        <div className="w-1/2 border-r border-gray-200 flex flex-col">
          <textarea
            value={content}
            onChange={handleChange}
            placeholder="Type markdown notes here..."
            className="flex-1 p-4 font-mono text-sm resize-none focus:outline-none focus:ring-0 leading-relaxed text-gray-800"
          />
        </div>

        {/* Live Preview Area */}
        <div className="w-1/2 p-6 overflow-y-auto bg-gray-50 prose prose-sm max-w-none">
          <div
            dangerouslySetInnerHTML={{ __html: getHtml() }}
            className="text-gray-900 leading-normal"
          />
        </div>
      </div>
    </div>
  );
};

