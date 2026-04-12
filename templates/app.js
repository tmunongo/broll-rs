function brollApp() {
  return {
    // Search
    query: '',
    results: [],
    loading: false,
    searched: false,
    selected: null,
    sources: [
      { id: 'pexels',  label: 'Pexels',  on: true },
      { id: 'pixabay', label: 'Pixabay', on: true },
      { id: 'archive', label: 'Archive', on: true },
      { id: 'youtube', label: 'YouTube', on: true },
    ],

    // Library
    library: [],
    activeProjectFilter: null,

    // Projects
    projects: [],
    showNewProject: false,
    newProjectName: '',

    // Download modal
    downloadModal: null,  // video pending project selection
    pendingProjectId: '',

    // Preview modal
    preview: null,

    // Download tracking
    dlStates: {},  // id -> 'active' | 'done' | 'error'
    pollTimers: {},

    // Toasts
    toasts: [],

    // ── Lifecycle ──────────────────────────────────────────────────────
    init() {
      this.activeProjectFilter = localStorage.getItem('broll:activeProject') || null;
      this.loadProjects();
      this.loadLibrary();
      // Auto-refresh while downloads are running
      setInterval(() => {
        const hasActive = Object.values(this.dlStates).some(s => s === 'active');
        if (hasActive) this.loadLibrary();
      }, 5000);
    },

    setActiveProject(id) {
      this.activeProjectFilter = id;
      if (id) {
        localStorage.setItem('broll:activeProject', id);
      } else {
        localStorage.removeItem('broll:activeProject');
      }
    },

    get filteredLibrary() {
      if (!this.activeProjectFilter) return this.library;
      return this.library.filter(v => v.project_id === this.activeProjectFilter);
    },

    // ── Search ────────────────────────────────────────────────────────
    async doSearch() {
      const q = this.query.trim();
      if (!q) return;
      this.loading = true;
      this.searched = true;
      this.results = [];
      try {
        const sources = this.sources.filter(s => s.on).map(s => s.id).join(',');
        const res = await fetch(`/api/search?q=${encodeURIComponent(q)}&sources=${sources}`);
        if (!res.ok) throw new Error(await res.text());
        this.results = await res.json();
      } catch (e) {
        this.toast('Search failed: ' + e.message, 'error');
      } finally {
        this.loading = false;
      }
    },

    clearSearch() {
      this.query = ''; this.results = [];
      this.searched = false; this.selected = null;
    },

    // ── Preview ───────────────────────────────────────────────────────
    openPreview(video) { this.preview = video; },

    // ── Download modal ────────────────────────────────────────────────
    openDownloadModal(video) {
      if (!video) return;
      if (this.dlStates[video.id] === 'active') return;
      this.downloadModal = video;
      // Pre-select the active project filter if set
      this.pendingProjectId = this.activeProjectFilter || '';
    },

    async confirmDownload() {
      const video = this.downloadModal;
      if (!video) return;
      this.downloadModal = null;
      this.dlStates[video.id] = 'active';
      this.toast('Download started: ' + video.title.slice(0, 40) + '…', 'info');

      try {
        const res = await fetch('/api/download', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            id: video.id,
            title: video.title,
            source: video.source,
            download_url: video.download_url,
            thumbnail: video.thumbnail,
            duration: video.duration,
            project_id: this.pendingProjectId || null,
          }),
        });
        if (!res.ok) throw new Error(await res.text());
        this.pendingProjectId = '';
        this.loadLibrary();
        this.pollStatus(video.id);
      } catch (e) {
        this.dlStates[video.id] = 'error';
        this.toast('Download failed: ' + e.message, 'error');
      }
    },

    pollStatus(id) {
      clearInterval(this.pollTimers[id]);
      this.pollTimers[id] = setInterval(async () => {
        try {
          const res = await fetch(`/api/download/status/${encodeURIComponent(id)}`);
          const data = await res.json();
          if (data.status === 'complete') {
            this.dlStates[id] = 'done';
            clearInterval(this.pollTimers[id]);
            this.toast('Download complete!', 'success');
            this.loadLibrary();
          } else if (data.status === 'error') {
            this.dlStates[id] = 'error';
            clearInterval(this.pollTimers[id]);
            this.toast('Download error.', 'error');
            this.loadLibrary();
          }
        } catch (_) {}
      }, 3000);
    },

    dlClass(id) {
      const s = this.dlStates[id];
      if (s === 'active') return 'dl-active';
      if (s === 'done')   return 'dl-done';
      if (s === 'error')  return 'dl-err';
      return '';
    },
    dlLabel(id) {
      const s = this.dlStates[id];
      if (s === 'active') return '⟳ Fetching';
      if (s === 'done')   return '✓ Saved';
      if (s === 'error')  return '✗ Error';
      return '↓ Save';
    },

    // ── Library ───────────────────────────────────────────────────────
    async loadLibrary() {
      try {
        const res = await fetch('/api/library');
        const data = await res.json();
        this.library = data;
        // Sync states from DB
        data.forEach(item => {
          if (item.status === 'complete')   this.dlStates[item.id] = 'done';
          if (item.status === 'error')      this.dlStates[item.id] = 'error';
          if (item.status === 'downloading' || item.status === 'pending')
            this.dlStates[item.id] = 'active';
        });
      } catch (_) {}
    },

    async deleteClip(id) {
      try {
        await fetch(`/api/library/${encodeURIComponent(id)}`, { method: 'DELETE' });
        delete this.dlStates[id];
        this.loadLibrary();
        this.toast('Clip removed', 'info');
      } catch (_) { this.toast('Delete failed', 'error'); }
    },

    async retryDownload(id) {
      this.dlStates[id] = 'active';
      this.toast('Retrying download…', 'info');
      try {
        const res = await fetch(`/api/download/retry/${encodeURIComponent(id)}`, {
          method: 'POST',
        });
        if (!res.ok) {
          const err = await res.json();
          throw new Error(err.error || 'Retry failed');
        }
        this.loadLibrary();
        this.pollStatus(id);
      } catch (e) {
        this.dlStates[id] = 'error';
        this.toast('Retry failed: ' + e.message, 'error');
      }
    },

    // ── Projects ──────────────────────────────────────────────────────
    async loadProjects() {
      try {
        const res = await fetch('/api/projects');
        this.projects = await res.json();
      } catch (_) {}
    },

    async createProject() {
      const name = this.newProjectName.trim();
      if (!name) return;
      try {
        const res = await fetch('/api/projects', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name }),
        });
        if (!res.ok) {
          const err = await res.json();
          throw new Error(err.error || 'Failed');
        }
        this.newProjectName = '';
        this.showNewProject = false;
        await this.loadProjects();
        this.toast('Project created: ' + name, 'success');
      } catch (e) {
        this.toast(e.message, 'error');
      }
    },

    async deleteProject(id) {
      try {
        await fetch(`/api/projects/${id}`, { method: 'DELETE' });
        if (this.activeProjectFilter === id) this.setActiveProject(null);
        await this.loadProjects();
        await this.loadLibrary();
        this.toast('Project deleted', 'info');
      } catch (_) { this.toast('Delete failed', 'error'); }
    },

    projectName(id) {
      return this.projects.find(p => p.id === id)?.name || '';
    },
    projectSlug(id) {
      return this.projects.find(p => p.id === id)?.slug || '';
    },

    // ── Utils ─────────────────────────────────────────────────────────
    fmtDur(seconds) {
      if (!seconds) return '';
      const s = Math.round(seconds);
      const m = Math.floor(s / 60);
      const h = Math.floor(m / 60);
      if (h > 0) return `${h}:${String(m % 60).padStart(2,'0')}:${String(s % 60).padStart(2,'0')}`;
      return `${m}:${String(s % 60).padStart(2,'0')}`;
    },

    toast(msg, type = 'info') {
      const id = Date.now();
      this.toasts.push({ id, msg, type });
      setTimeout(() => { this.toasts = this.toasts.filter(t => t.id !== id); }, 4000);
    },
  };
}
