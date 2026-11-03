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
    onVoteDelete: vi.fn(),
    onUnvoteDelete: vi.fn(),
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

  it('renders delete vote count badge when marked for deletion', () => {
    const votedFile: FileEntry = {
      name: 'Old-Exam.pdf',
      hash: 'jkl3456',
      size_bytes: 524288,
      is_dir: false,
      sync_state: 'cached',
      delete_votes: ['peer1', 'peer2'],
    };

    render(<FileRow file={votedFile} {...dummyProps} />);

    expect(screen.getByText(/2\/3 delete votes/i)).toBeInTheDocument();
  });

  it('renders seed count badge with rare indicator when seeds <= 1', () => {
    const rareFile: FileEntry = {
      name: 'Rare-Manuscript.pdf',
      hash: 'rare123',
      size_bytes: 1048576,
      is_dir: false,
      sync_state: 'cloud_only',
      seeds_count: 1,
    };

    render(<FileRow file={rareFile} {...dummyProps} />);

    expect(screen.getByText(/1 seed \(rare\)/i)).toBeInTheDocument();
  });

  it('renders folder row with item count and folder badge', () => {
    const folderEntry: FileEntry = {
      name: 'ComputerScience',
      hash: 'folder-cs',
      size_bytes: 4194304,
      is_dir: true,
      sync_state: 'cached',
      item_count: 5,
    };

    render(<FileRow file={folderEntry} {...dummyProps} />);

    expect(screen.getByText('ComputerScience')).toBeInTheDocument();
    expect(screen.getByText(/5 items/i)).toBeInTheDocument();
    expect(screen.getByText('Folder')).toBeInTheDocument();
  });
});
