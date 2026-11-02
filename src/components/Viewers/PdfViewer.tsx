import React, { useEffect, useRef, useState } from 'react';
import * as pdfjsLib from 'pdfjs-dist';
import { ZoomIn, ZoomOut, ChevronLeft, ChevronRight, Loader2 } from 'lucide-react';

// Configure pdfjs worker
pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).href;

interface PdfViewerProps {
  data: Uint8Array | null;
  onClose?: () => void;
}

export const PdfViewer: React.FC<PdfViewerProps> = ({ data, onClose }) => {
  const [pdfDoc, setPdfDoc] = useState<pdfjsLib.PDFDocumentProxy | null>(null);
  const [pageNum, setPageNum] = useState<number>(1);
  const [totalPages, setTotalPages] = useState<number>(0);
  const [scale, setScale] = useState<number>(1.2);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const renderTaskRef = useRef<any>(null);

  useEffect(() => {
    if (!data || data.length === 0) {
      setLoading(false);
      setError('No PDF data provided');
      return;
    }

    setLoading(true);
    setError(null);

    const loadingTask = pdfjsLib.getDocument({ data });
    loadingTask.promise
      .then((doc) => {
        setPdfDoc(doc);
        setTotalPages(doc.numPages);
        setPageNum(1);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Error loading PDF:', err);
        setError('Failed to parse PDF document.');
        setLoading(false);
      });

    return () => {
      loadingTask.destroy();
    };
  }, [data]);

  useEffect(() => {
    if (!pdfDoc || !canvasRef.current) return;

    if (renderTaskRef.current) {
      renderTaskRef.current.cancel();
    }

    pdfDoc.getPage(pageNum).then((page) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const context = canvas.getContext('2d');
      if (!context) return;

      const viewport = page.getViewport({ scale });
      canvas.height = viewport.height;
      canvas.width = viewport.width;

      const renderContext = {
        canvasContext: context,
        viewport,
      };

      const renderTask = page.render(renderContext);
      renderTaskRef.current = renderTask;

      renderTask.promise.catch((err) => {
        if (err?.name !== 'RenderingCancelledException') {
          console.error('Render page error:', err);
        }
      });
    });
  }, [pdfDoc, pageNum, scale]);

  const handlePrevPage = () => {
    if (pageNum > 1) setPageNum(pageNum - 1);
  };

  const handleNextPage = () => {
    if (pdfDoc && pageNum < pdfDoc.numPages) setPageNum(pageNum + 1);
  };

  return (
    <div className="flex flex-col h-full bg-gray-100">
      {/* Top control bar */}
      <div className="flex items-center justify-between px-4 py-2 bg-white border-b border-gray-200">
        <div className="flex items-center gap-2">
          <button
            onClick={handlePrevPage}
            disabled={pageNum <= 1}
            className="p-1.5 rounded hover:bg-gray-100 disabled:opacity-40"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>
          <span className="text-sm font-medium text-gray-700">
            Page {pageNum} of {totalPages || 1}
          </span>
          <button
            onClick={handleNextPage}
            disabled={pageNum >= totalPages}
            className="p-1.5 rounded hover:bg-gray-100 disabled:opacity-40"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setScale((s) => Math.max(0.5, s - 0.2))}
            className="p-1.5 rounded hover:bg-gray-100"
            title="Zoom Out"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <span className="text-xs text-gray-500 w-12 text-center">
            {Math.round(scale * 100)}%
          </span>
          <button
            onClick={() => setScale((s) => Math.min(2.5, s + 0.2))}
            className="p-1.5 rounded hover:bg-gray-100"
            title="Zoom In"
          >
            <ZoomIn className="w-4 h-4" />
          </button>

          {onClose && (
            <button
              onClick={onClose}
              className="ml-4 px-3 py-1 text-xs bg-gray-200 hover:bg-gray-300 rounded font-medium"
            >
              Close
            </button>
          )}
        </div>
      </div>

      {/* Viewer content */}
      <div className="flex-1 overflow-auto flex items-center justify-center p-6">
        {loading && (
          <div className="flex flex-col items-center gap-2 text-gray-500">
            <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
            <p className="text-sm">Rendering PDF slides...</p>
          </div>
        )}

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded text-sm">
            {error}
          </div>
        )}

        <canvas
          ref={canvasRef}
          className={`shadow-lg bg-white rounded transition-opacity ${
            loading ? 'opacity-0' : 'opacity-100'
          }`}
        />
      </div>
    </div>
  );
};

