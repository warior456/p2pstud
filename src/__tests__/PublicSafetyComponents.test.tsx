import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SecurityDetailsModal } from '../components/Modals/SecurityDetailsModal';
import { FileRow } from '../components/Explorer/FileRow';
import { CacheSlider } from '../components/Settings/CacheSlider';
import { FileEntry, StorageStats } from '../types';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Public Safety UI Components', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe('SecurityDetailsModal', () => {
    const quarantinedFile: FileEntry = {
      name: 'fake_exam.pdf',
      hash: 'deadbeef1234567890abcdef',
      size_bytes: 1024,
      is_dir: false,
      sync_state: 'cloud_only',
      is_quarantined: true,
      quarantine_report_count: 5,
    };

    it('fetches and displays quarantine reasons and handles Block & Purge', async () => {
      mockInvoke.mockResolvedValueOnce({
        is_quarantined: true,
        report_count: 5,
        reasons: ['Disguised trojan executable', 'Phishing link inside'],
      });

      const onClose = vi.fn();
      const onBlockFile = vi.fn();
      const onReportDangerous = vi.fn();

      render(
        <SecurityDetailsModal
          isOpen={true}
          file={quarantinedFile}
          onClose={onClose}
          onBlockFile={onBlockFile}
          onReportDangerous={onReportDangerous}
        />
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_quarantine_info', {
          hash: quarantinedFile.hash,
        });
      });

      expect(await screen.findByText('Disguised trojan executable')).toBeInTheDocument();
      expect(screen.getByText('Phishing link inside')).toBeInTheDocument();

      fireEvent.click(screen.getByText(/Block & Purge Locally/i));
      expect(onClose).toHaveBeenCalled();
      expect(onBlockFile).toHaveBeenCalledWith(quarantinedFile.hash);
    });

    it('allows submitting a new safety report from the modal', async () => {
      mockInvoke.mockResolvedValueOnce({
        is_quarantined: false,
        report_count: 2,
        reasons: ['Suspicious macro'],
      });

      const onClose = vi.fn();
      const onReportDangerous = vi.fn();

      render(
        <SecurityDetailsModal
          isOpen={true}
          file={{ ...quarantinedFile, is_quarantined: false, quarantine_report_count: 2 }}
          onClose={onClose}
          onReportDangerous={onReportDangerous}
        />
      );

      await waitFor(() => expect(mockInvoke).toHaveBeenCalled());

      fireEvent.click(screen.getByText(/Submit your own report/i));
      const textarea = screen.getByPlaceholderText(/Describe the threat/i);
      fireEvent.change(textarea, { target: { value: 'Contains credential harvester' } });
      fireEvent.click(screen.getByText('Submit Report'));

      expect(onReportDangerous).toHaveBeenCalledWith(
        quarantinedFile.hash,
        'Contains credential harvester'
      );
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe('FileRow Security Badges', () => {
    const baseProps = {
      isSelected: false,
      onClick: vi.fn(),
      onPin: vi.fn(),
      onUnpin: vi.fn(),
      onDownload: vi.fn(),
      onPreview: vi.fn(),
    };

    it('renders early-warning Flagged badge and triggers onInspectSecurity on click', () => {
      const flaggedFile: FileEntry = {
        name: 'suspicious_notes.pdf',
        hash: 'flaggedhash123',
        size_bytes: 512,
        is_dir: false,
        sync_state: 'cloud_only',
        is_quarantined: false,
        quarantine_report_count: 2,
      };
      const onInspectSecurity = vi.fn();

      render(
        <FileRow
          file={flaggedFile}
          {...baseProps}
          onInspectSecurity={onInspectSecurity}
        />
      );

      const badge = screen.getByText(/⚠️ Flagged \(2\)/i);
      expect(badge).toBeInTheDocument();
      fireEvent.click(badge);
      expect(onInspectSecurity).toHaveBeenCalledWith(flaggedFile);
    });
  });

  describe('CacheSlider Locally Blocked Content & Zero-Trust Guard', () => {
    const mockStats: StorageStats = {
      used_bytes: 1024 * 1024 * 1024,
      quota_bytes: 5 * 1024 * 1024 * 1024,
      pinned_bytes: 0,
      peer_count: 1,
      full_mirror: false,
      upload_bytes_per_sec: 0,
      download_bytes_per_sec: 0,
    };

    it('displays blocked hashes and unblocks them via unblock_file IPC', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_blocked_files_detailed') {
          return Promise.resolve([
            {
              hash: 'abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
              blocked_at: 1730000000,
              reason: 'malware',
            },
          ]);
        }
        return Promise.resolve(undefined);
      });

      render(<CacheSlider stats={mockStats} />);

      expect(await screen.findByText(/1 blocked/i)).toBeInTheDocument();
      expect(screen.getByText(/malware/i)).toBeInTheDocument();

      const unblockBtn = screen.getByRole('button', { name: /Unblock/i });
      fireEvent.click(unblockBtn);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('unblock_file', {
          hash: 'abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
        });
      });

      expect(await screen.findByText(/0 blocked/i)).toBeInTheDocument();
    });
  });
});
