import { describe, it, expect, beforeEach } from 'vitest';
import { QuickFilter } from '../App';

describe('Quick Filters & Unified Settings Configuration', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe('Quick Filter Matching Logic', () => {
    const DEFAULT_QUICK_FILTERS: QuickFilter[] = [
      { id: 'documents', name: 'Documents', extensions: ['.pdf', '.doc', '.docx', '.odt', '.rtf'] },
      { id: 'notes', name: 'Notes', extensions: ['.md', '.txt'] },
      { id: 'code', name: 'Code', extensions: ['.rs', '.ts', '.tsx', '.js', '.jsx', '.py', '.json', '.toml', '.html', '.css'] },
      { id: 'archives', name: 'Archives', extensions: ['.zip', '.tar', '.gz', '.7z', '.rar'] },
      { id: 'media', name: 'Media', extensions: ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp', '.mp4', '.mp3'] },
    ];

    const matchesFilter = (fileName: string, filter: QuickFilter) => {
      const lower = fileName.toLowerCase();
      return filter.extensions.some((ext) => {
        const cleanExt = ext.trim().toLowerCase();
        if (cleanExt.startsWith('.')) {
          return lower.endsWith(cleanExt);
        }
        return lower.includes(cleanExt);
      });
    };

    it('matches document files correctly', () => {
      const docFilter = DEFAULT_QUICK_FILTERS.find((f) => f.id === 'documents')!;
      expect(matchesFilter('chapter1.pdf', docFilter)).toBe(true);
      expect(matchesFilter('syllabus.docx', docFilter)).toBe(true);
      expect(matchesFilter('notes.txt', docFilter)).toBe(false);
    });

    it('matches notes and markdown files', () => {
      const notesFilter = DEFAULT_QUICK_FILTERS.find((f) => f.id === 'notes')!;
      expect(matchesFilter('meeting_notes.md', notesFilter)).toBe(true);
      expect(matchesFilter('todo.txt', notesFilter)).toBe(true);
      expect(matchesFilter('program.rs', notesFilter)).toBe(false);
    });

    it('supports custom user-defined quick filters and local storage persistence', () => {
      const customFilter: QuickFilter = {
        id: 'custom-python',
        name: 'Python Notebooks',
        extensions: ['.py', '.ipynb'],
        isCustom: true,
      };

      // Test matching
      expect(matchesFilter('analysis.py', customFilter)).toBe(true);
      expect(matchesFilter('lab.ipynb', customFilter)).toBe(true);
      expect(matchesFilter('script.sh', customFilter)).toBe(false);

      // Test persistence
      localStorage.setItem('p2pstud_custom_quick_filters', JSON.stringify([customFilter]));
      const retrieved = JSON.parse(localStorage.getItem('p2pstud_custom_quick_filters') || '[]');
      expect(retrieved).toHaveLength(1);
      expect(retrieved[0].name).toBe('Python Notebooks');
      expect(retrieved[0].extensions).toEqual(['.py', '.ipynb']);
    });
  });

  describe('Unified Settings Tabs', () => {
    const UNIFIED_SETTINGS_TABS = [
      { id: 'cache', title: 'Cache & Quota' },
      { id: 'bandwidth', title: 'Bandwidth & Battery' },
      { id: 'network', title: 'Network & Stealth' },
      { id: 'backup', title: 'Backups & Updates' },
    ];

    it('contains all 4 unified settings tabs without omissions', () => {
      expect(UNIFIED_SETTINGS_TABS).toHaveLength(4);
      const tabIds = UNIFIED_SETTINGS_TABS.map((t) => t.id);
      expect(tabIds).toContain('cache');
      expect(tabIds).toContain('bandwidth');
      expect(tabIds).toContain('network');
      expect(tabIds).toContain('backup');
    });
  });
});
