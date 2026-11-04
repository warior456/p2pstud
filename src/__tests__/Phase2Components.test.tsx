import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { OfflineBanner } from '../components/OfflineBanner';
import { SoleSeedBanner } from '../components/SoleSeedBanner';
import { CredentialsStep } from '../components/Onboarding/CredentialsStep';
import { ConnectionDoctor } from '../components/Onboarding/ConnectionDoctor';
import { FileRow } from '../components/Explorer/FileRow';
import { FileEntry } from '../types';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe('Phase 2 Components Test Suite', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('OfflineBanner', () => {
    it('does not render when isOnline is true', () => {
      const { container } = render(<OfflineBanner isOnline={true} />);
      expect(container.firstChild).toBeNull();
    });

    it('renders alert when offline and triggers reconnect', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);
      render(<OfflineBanner isOnline={false} />);

      expect(screen.getByText(/Offline — viewing cached files only/i)).toBeInTheDocument();
      const retryBtn = screen.getByRole('button', { name: /Retry Connection/i });
      fireEvent.click(retryBtn);

      expect(invoke).toHaveBeenCalledWith('reconnect_swarm');
    });
  });

  describe('SoleSeedBanner', () => {
    it('renders sole seed warning when peer is only seeder', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([
        { hash: 'rarehash1', name: 'RareNotes.pdf', size_bytes: 1024 },
      ]);

      render(<SoleSeedBanner />);

      await waitFor(() => {
        expect(screen.getByText(/You are the only online copy/i)).toBeInTheDocument();
      });
    });

    it('can be dismissed for the session', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([
        { hash: 'rarehash1', name: 'RareNotes.pdf', size_bytes: 1024 },
      ]);

      render(<SoleSeedBanner />);

      await waitFor(() => {
        expect(screen.getByText(/You are the only online copy/i)).toBeInTheDocument();
      });

      const dismissBtn = screen.getByTitle(/Dismiss alert for this session/i);
      fireEvent.click(dismissBtn);

      expect(screen.queryByText(/You are the only online copy/i)).toBeNull();
    });
  });

  describe('CredentialsStep (Standalone Decentralization Boundary)', () => {
    it('renders standalone mode and continues with passphrase', () => {
      const onComplete = vi.fn();
      render(<CredentialsStep onComplete={onComplete} />);

      expect(screen.getByText(/Study Group Passphrase/i)).toBeInTheDocument();
      expect(screen.getByText(/Standalone P2P Mode:/i)).toBeInTheDocument();

      const input = screen.getByPlaceholderText(/e.g. compsci-2026-fall/i) as HTMLInputElement;
      fireEvent.change(input, { target: { value: 'my-custom-study-swarm' } });

      const continueBtn = screen.getByRole('button', { name: /Continue/i });
      fireEvent.click(continueBtn);

      expect(onComplete).toHaveBeenCalledWith('my-custom-study-swarm');
    });
  });

  describe('FileRow Study Ratings & Quarantine', () => {
    const dummyProps = {
      isSelected: false,
      onClick: vi.fn(),
      onPin: vi.fn(),
      onUnpin: vi.fn(),
      onDownload: vi.fn(),
      onPreview: vi.fn(),
      onRate: vi.fn(),
      onToggleStar: vi.fn(),
      onReportDangerous: vi.fn(),
    };

    it('renders quarantined badge and strikes through name', () => {
      const quarantinedFile: FileEntry = {
        name: 'sketchy-script.py',
        hash: 'malicious123',
        size_bytes: 2048,
        is_dir: false,
        sync_state: 'cloud_only',
        is_quarantined: true,
      };

      render(<FileRow file={quarantinedFile} {...dummyProps} />);

      expect(screen.getByText(/⛔ Quarantined/i)).toBeInTheDocument();
      expect(screen.getByText('sketchy-script.py')).toHaveClass('line-through');
    });

    it('calls onRate and onToggleStar', () => {
      const file: FileEntry = {
        name: 'StudyNotes.pdf',
        hash: 'goodnotes123',
        size_bytes: 4096,
        is_dir: false,
        sync_state: 'cached',
        net_rating: 4,
        is_starred: true,
      };

      render(<FileRow file={file} {...dummyProps} />);

      expect(screen.getByText('+4')).toBeInTheDocument();

      const upvoteBtn = screen.getByTitle(/Helpful study material/i);
      fireEvent.click(upvoteBtn);
      expect(dummyProps.onRate).toHaveBeenCalledWith('goodnotes123', 1);

      const downvoteBtn = screen.getByTitle(/Low quality/i);
      fireEvent.click(downvoteBtn);
      expect(dummyProps.onRate).toHaveBeenCalledWith('goodnotes123', -1);

      const starBtn = screen.getByTitle(/Remove star/i);
      fireEvent.click(starBtn);
      expect(dummyProps.onToggleStar).toHaveBeenCalledWith('goodnotes123');
    });

    it('prompts confirmation warning before downloading dangerous executable', () => {
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
      const exeFile: FileEntry = {
        name: 'install-tools.exe',
        hash: 'exehash999',
        size_bytes: 1048576,
        is_dir: false,
        sync_state: 'cloud_only',
      };

      render(<FileRow file={exeFile} {...dummyProps} />);

      // Open context menu
      const moreBtn = screen.getByRole('button', { name: '' });
      fireEvent.click(moreBtn);

      const downloadBtn = screen.getByText('Download');
      fireEvent.click(downloadBtn);

      expect(confirmSpy).toHaveBeenCalledWith(expect.stringContaining('SECURITY WARNING'));
      expect(dummyProps.onDownload).not.toHaveBeenCalled();

      confirmSpy.mockRestore();
    });
  });

  describe('ConnectionDoctor', () => {
    it('renders all 4 diagnostic test names and invokes runner', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);
      render(<ConnectionDoctor />);

      expect(screen.getByText(/Campus Local Discovery \(mDNS\)/i)).toBeInTheDocument();
      expect(screen.getByText(/Direct UDP \/ QUIC/i)).toBeInTheDocument();
      expect(screen.getByText(/Encrypted Relay \(Port 443 TCP\)/i)).toBeInTheDocument();
      expect(screen.getByText(/NAT Architecture Inspection/i)).toBeInTheDocument();

      expect(invoke).toHaveBeenCalledWith('run_connection_doctor');
    });
  });
});
