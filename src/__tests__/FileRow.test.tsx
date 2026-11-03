import { render, screen, fireEvent } from '@testing-library/react';
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

    expect(screen.getByText(/2\s*🗑️/)).toBeInTheDocument();
    expect(screen.getByText(/Defend/i)).toBeInTheDocument();
  });

  it('renders pending deletion grace period warning badge when scheduled', () => {
    const pendingFile: FileEntry = {
      name: 'Soon-Deleted.pdf',
      hash: 'sched999',
      size_bytes: 524288,
      is_dir: false,
      sync_state: 'cloud_only',
      delete_votes: ['peer1', 'peer2', 'peer3'],
      deletion_scheduled_at: Math.floor(Date.now() / 1000) + 7200,
    };

    render(<FileRow file={pendingFile} {...dummyProps} />);

    expect(screen.getByText(/Deleting in/i)).toBeInTheDocument();
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

  it('renders move to folder action in context menu and calls onMove', () => {
    const onMove = vi.fn();
    const testFile: FileEntry = {
      name: 'AlgorithmNotes.pdf',
      hash: 'algo999',
      size_bytes: 1048576,
      is_dir: false,
      sync_state: 'cached',
    };

    render(<FileRow file={testFile} {...dummyProps} onMove={onMove} />);

    // Open context menu
    const menuBtn = screen.getByRole('button');
    fireEvent.click(menuBtn);

    const moveOption = screen.getByText(/Move to Folder\.\.\./i);
    expect(moveOption).toBeInTheDocument();
    fireEvent.click(moveOption);

    expect(onMove).toHaveBeenCalledWith('algo999');
  });

  it('shows sole seed protection and prevents local eviction when seeds <= 1', () => {
    const alertMock = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const onEvictLocal = vi.fn();
    const soleSeedFile: FileEntry = {
      name: 'Sole-Copy.pdf',
      hash: 'sole111',
      size_bytes: 2048576,
      is_dir: false,
      sync_state: 'cached',
      seeds_count: 1, // Sole active seed!
    };

    render(<FileRow file={soleSeedFile} {...dummyProps} onEvictLocal={onEvictLocal} />);

    // Open context menu
    const menuBtn = screen.getByRole('button');
    fireEvent.click(menuBtn);

    const evictBtn = screen.getByText(/Free Local Disk Space/i);
    expect(screen.getByText('Sole Seed')).toBeInTheDocument();

    fireEvent.click(evictBtn);

    // Eviction must be blocked and alert shown!
    expect(alertMock).toHaveBeenCalledWith(
      expect.stringContaining('You are currently the ONLY active seed in the swarm')
    );
    expect(onEvictLocal).not.toHaveBeenCalled();

    alertMock.mockRestore();
  });
});
