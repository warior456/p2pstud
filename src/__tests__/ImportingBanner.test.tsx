import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ImportingBanner } from '../components/ImportingBanner';
import { ImportProgress } from '../types';

describe('ImportingBanner Component', () => {
  it('returns null when progress is null', () => {
    const { container } = render(<ImportingBanner progress={null} onDismiss={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders importing status, file count, and disk speed during active import', () => {
    const progress: ImportProgress = {
      archive_name: 'semester1.zip',
      current_entry: 25,
      total_entries: 50,
      bytes_processed: 52428800, // 50 MB
      total_bytes: 104857600, // 100 MB
      current_file: 'notes/week1.pdf',
      bytes_per_sec: 15728640, // 15 MB/s
      status: 'importing',
    };

    render(<ImportingBanner progress={progress} onDismiss={vi.fn()} />);

    expect(screen.getByText(/Importing: semester1.zip/i)).toBeInTheDocument();
    expect(screen.getByText(/50%/i)).toBeInTheDocument();
    expect(screen.getByText(/25 \/ 50 entries/i)).toBeInTheDocument();
    expect(screen.getByText(/15.0 MB\/s \(local disk\)/i)).toBeInTheDocument();
    expect(screen.getByText(/notes\/week1.pdf/i)).toBeInTheDocument();
  });

  it('renders completion state upon successful archive unpacking', () => {
    const progress: ImportProgress = {
      archive_name: 'semester1.zip',
      current_entry: 50,
      total_entries: 50,
      bytes_processed: 104857600,
      total_bytes: 104857600,
      current_file: 'Done',
      bytes_per_sec: 0,
      status: 'completed',
    };

    render(<ImportingBanner progress={progress} onDismiss={vi.fn()} />);

    expect(screen.getByText(/Import Complete: semester1.zip/i)).toBeInTheDocument();
    expect(screen.getByText(/50 files successfully imported/i)).toBeInTheDocument();
  });

  it('triggers onDismiss when close button is clicked', () => {
    const onDismiss = vi.fn();
    const progress: ImportProgress = {
      archive_name: 'course.zip',
      current_entry: 1,
      total_entries: 1,
      bytes_processed: 1000,
      total_bytes: 1000,
      current_file: 'test.pdf',
      bytes_per_sec: 0,
      status: 'completed',
    };

    render(<ImportingBanner progress={progress} onDismiss={onDismiss} />);

    const closeBtn = screen.getByTitle(/Dismiss import bar/i);
    fireEvent.click(closeBtn);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});

