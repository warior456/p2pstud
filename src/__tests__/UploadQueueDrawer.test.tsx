import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { UploadQueueDrawer } from '../components/Explorer/UploadQueueDrawer';
import { UploadItem } from '../types';

describe('UploadQueueDrawer Component', () => {
  const mockQueue: UploadItem[] = [
    {
      id: 'item-1',
      name: 'LectureNotes.pdf',
      size_bytes: 5242880,
      progress: 65,
      status: 'uploading',
    },
    {
      id: 'item-2',
      name: 'Dataset.csv',
      size_bytes: 1048576,
      progress: 100,
      status: 'completed',
    },
  ];

  it('renders nothing when upload queue is empty', () => {
    const { container } = render(
      <UploadQueueDrawer queue={[]} onDismiss={vi.fn()} onClearAll={vi.fn()} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders active uploads and file items', () => {
    render(
      <UploadQueueDrawer queue={mockQueue} onDismiss={vi.fn()} onClearAll={vi.fn()} />
    );

    expect(screen.getByText(/Uploading 1 file\.\.\./i)).toBeInTheDocument();
    expect(screen.getByText('LectureNotes.pdf')).toBeInTheDocument();
    expect(screen.getByText('Dataset.csv')).toBeInTheDocument();
    expect(screen.getByText(/65%/i)).toBeInTheDocument();
    expect(screen.getByText(/100%/i)).toBeInTheDocument();
  });

  it('calls onDismiss when clicking dismiss button on item', () => {
    const onDismiss = vi.fn();
    render(
      <UploadQueueDrawer queue={mockQueue} onDismiss={onDismiss} onClearAll={vi.fn()} />
    );

    const dismissButtons = screen.getAllByTitle('Dismiss');
    fireEvent.click(dismissButtons[0]);

    expect(onDismiss).toHaveBeenCalledWith('item-1');
  });

  it('calls onClearAll when clicking dismiss all in header', () => {
    const onClearAll = vi.fn();
    render(
      <UploadQueueDrawer queue={mockQueue} onDismiss={vi.fn()} onClearAll={onClearAll} />
    );

    const dismissAllBtn = screen.getByTitle('Dismiss all');
    fireEvent.click(dismissAllBtn);

    expect(onClearAll).toHaveBeenCalled();
  });
});

