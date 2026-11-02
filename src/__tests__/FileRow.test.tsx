import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FileRow } from '../components/Explorer/FileRow';
import { FileEntry } from '../types';

describe('FileRow Component', () => {
  const dummyProps = {
    isSelected: false,
    onClick: vi.fn(),
    onPin: vi.fn(),
    onUnpin: vi.fn(),
    onDownload: vi.fn(),
    onPreview: vi.fn(),
  };

  it('renders pinned badge correctly', () => {
    const pinnedFile: FileEntry = {
      name: 'Important-Lecture.pdf',
      hash: 'abc1234',
      size_bytes: 1048576,
      is_dir: false,
      sync_state: 'pinned',
    };

    render(<FileRow file={pinnedFile} {...dummyProps} />);

    expect(screen.getByText('Important-Lecture.pdf')).toBeInTheDocument();
    expect(screen.getByText('Pinned')).toBeInTheDocument();
  });

  it('renders cloud_only badge correctly', () => {
    const cloudFile: FileEntry = {
      name: 'Uncached-Slides.pdf',
      hash: 'def5678',
      size_bytes: 2097152,
      is_dir: false,
      sync_state: 'cloud_only',
    };

    render(<FileRow file={cloudFile} {...dummyProps} />);

    expect(screen.getByText('Uncached-Slides.pdf')).toBeInTheDocument();
    expect(screen.getByText('Cloud Only')).toBeInTheDocument();
  });

  it('renders seeding badge correctly', () => {
    const cachedFile: FileEntry = {
      name: 'Shared-Notes.md',
      hash: 'ghi9012',
      size_bytes: 4096,
      is_dir: false,
      sync_state: 'cached',
    };

    render(<FileRow file={cachedFile} {...dummyProps} />);

    expect(screen.getByText('Shared-Notes.md')).toBeInTheDocument();
    expect(screen.getByText('Seeding')).toBeInTheDocument();
  });
});

