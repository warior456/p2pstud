import React from 'react';
import { Home, ChevronRight } from 'lucide-react';

interface BreadcrumbsProps {
  paths: string[];
  onNavigate: (index: number) => void;
}

export const Breadcrumbs: React.FC<BreadcrumbsProps> = ({ paths, onNavigate }) => {
  return (
    <nav className="flex items-center space-x-1 text-sm text-gray-600 bg-gray-50 px-4 py-2 border-b border-gray-200">
      <button
        onClick={() => onNavigate(-1)}
        className="flex items-center text-gray-600 hover:text-blue-600 transition-colors p-1 rounded hover:bg-gray-200"
        title="Root workspace"
      >
        <Home className="w-4 h-4" />
      </button>

      {paths.map((segment, index) => (
        <React.Fragment key={index}>
          <ChevronRight className="w-4 h-4 text-gray-400" />
          <button
            onClick={() => onNavigate(index)}
            className={`px-1.5 py-0.5 rounded transition-colors ${
              index === paths.length - 1
                ? 'font-semibold text-gray-900 bg-gray-200 cursor-default'
                : 'text-gray-600 hover:text-blue-600 hover:bg-gray-200'
            }`}
          >
            {segment}
          </button>
        </React.Fragment>
      ))}
    </nav>
  );
};

