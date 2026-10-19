/* app.js — iap-web SolverForge UI */

(async function () {
  'use strict';

  var SLOT_MINUTES = 60;
  var DEFAULT_VIEWPORT_SLOTS = 12;
  var TIMELINE_TONES = ['emerald', 'blue', 'amber', 'rose', 'violet', 'slate'];

  var config = await fetch('/sf-config.json').then(function (response) {
    return response.json();
  });
  var uiModel = await fetch('/generated/ui-model.json').then(
    function (response) {
      return response.json();
    }
  );
  await IAP.init();
  uiModel.views = IAP.views;
  // Constraint dots come from the backend analysis of a real solution, never
  // from this metadata file (it can lag the model after a constraint change).
  delete uiModel.constraints;
  var app = document.getElementById('sf-app');
  var backend = SF.createBackend({ baseUrl: '' });
  var statusBar = SF.createStatusBar({
    constraints: [],
    onConstraintClick: function () {
      openAnalysis();
    }
  });
  var dataLoading = true;
  var controlPending = false;
  var sourcePlan = null;
  var currentPlan = null;
  var lastAnalysis = null;
  var bootstrapError = null;
  var demoCatalog = { defaultId: null, availableIds: [] };
  var activeTab =
    uiModel.views && uiModel.views.length ? uiModel.views[0].id : 'overview';
  var viewPanels = {};
  var viewTimelines = {};
  var sharedViewport = null;

  var tabs = (uiModel.views || []).map(function (view, index) {
    return {
      id: view.id,
      label: view.label,
      icon: view.kind === 'list' ? 'fa-list-ol' : 'fa-table-cells-large',
      active: index === 0
    };
  });
  if (!tabs.length) {
    tabs.push({
      id: 'overview',
      label: 'Overview',
      icon: 'fa-compass',
      active: true
    });
  }

  var header = SF.createHeader({
    logo: '/sf/img/ouroboros.svg',
    title: config.title,
    subtitle: config.subtitle,
    tabs: tabs,
    actions: {
      onSolve: function () {
        loadAndSolve();
      },
      onPause: function () {
        pauseSolve();
      },
      onResume: function () {
        resumeSolve();
      },
      onCancel: function () {
        cancelSolve();
      },
      onAnalyze: function () {
        openAnalysis();
      }
    },
    onTabChange: function (tab) {
      activeTab = tab;
      Object.keys(viewPanels).forEach(function (key) {
        viewPanels[key].style.display = key === tab ? '' : 'none';
      });
      overviewPanel.style.display = tab === 'overview' ? '' : 'none';
      if (currentPlan) renderViews(currentPlan);
      syncLifecycleMarkers();
    }
  });
  app.appendChild(header);
  tabs.forEach(function (tab) {
    header.sfControls.nav
      .querySelector('[data-tab="' + tab.id + '"]')
      .setAttribute('aria-label', tab.label);
  });
  [
    ['solveBtn', 'Solve'],
    ['pauseBtn', 'Pause'],
    ['resumeBtn', 'Resume'],
    ['cancelBtn', 'Stop'],
    ['analyzeBtn', 'Analyze score']
  ].forEach(function (entry) {
    header.sfControls[entry[0]].setAttribute('aria-label', entry[1]);
  });
  statusBar.bindHeader(header);
  app.appendChild(statusBar.el);

  var bootstrapNotice = SF.el('div', {
    className: 'sf-content',
    style: {
      display: 'none',
      padding: '16px',
      marginBottom: '16px',
      borderRadius: '12px',
      border: '1px solid #dc2626',
      background: '#fef2f2',
      color: '#991b1b'
    }
  });
  app.appendChild(bootstrapNotice);

  var overviewPanel = SF.el('div', {
    className: 'sf-content',
    style: { display: activeTab === 'overview' ? '' : 'none' }
  });
  var overviewContainer = SF.el('div', { id: 'sf-overview' });
  overviewPanel.appendChild(overviewContainer);
  app.appendChild(overviewPanel);

  (uiModel.views || []).forEach(function (view) {
    var panel = SF.el('div', {
      className: 'sf-content',
      style: { display: activeTab === view.id ? '' : 'none' }
    });
    panel.appendChild(SF.el('div', { id: 'view-' + view.id }));
    viewPanels[view.id] = panel;
    app.appendChild(panel);
  });

  app.appendChild(
    SF.createFooter({
      links: [
        { label: 'SolverForge', url: 'https://www.solverforge.org' },
        { label: 'Docs', url: 'https://www.solverforge.org/docs' }
      ]
    })
  );

  var analysisModal = SF.createModal({
    title: 'Score Analysis',
    width: '700px'
  });
  var controls;
  var solver = SF.createSolver({
    backend: backend,
    statusBar: statusBar,
    onProgress: function (meta) {
      syncLifecycleMarkers(meta);
    },
    onPauseRequested: function (meta) {
      syncLifecycleMarkers(meta);
    },
    onSolution: function (snapshot, meta) {
      if (snapshot && snapshot.solution) {
        renderAll(snapshot.solution);
      }
      syncLifecycleMarkers(meta);
    },
    onPaused: function (snapshot, meta) {
      if (snapshot && snapshot.solution) {
        renderAll(snapshot.solution);
      }
      syncLifecycleMarkers(meta);
    },
    onResumed: function (meta) {
      syncLifecycleMarkers(meta);
    },
    onCancelled: function (snapshot, meta) {
      if (snapshot && snapshot.solution) {
        renderAll(snapshot.solution);
      }
      syncLifecycleMarkers(meta);
    },
    onComplete: function (snapshot, meta) {
      if (snapshot && snapshot.solution) {
        renderAll(snapshot.solution);
      }
      syncLifecycleMarkers(meta);
    },
    onFailure: function (message, meta, snapshot, analysis) {
      if (snapshot && snapshot.solution) {
        renderAll(snapshot.solution);
      }
      if (analysis) {
        lastAnalysis = analysis;
      }
      console.error('Solver job failed:', message);
      syncLifecycleMarkers(meta);
    },
    onAnalysis: function (analysis) {
      lastAnalysis = analysis;
      syncLifecycleMarkers();
    },
    onError: function (message) {
      console.error('Solver lifecycle failed:', message);
      syncLifecycleMarkers();
    }
  });
  controls = IAP.mount({
    after: statusBar.el,
    render: function () {
      if (currentPlan) renderViews(currentPlan);
    },
    sync: function () {
      syncLifecycleMarkers();
    },
    viewport: function (viewport) {
      sharedViewport = viewport;
      Object.keys(viewTimelines).forEach(function (key) {
        viewTimelines[key].setViewport(viewport);
      });
    },
    load: function (id) {
      dataLoading = true;
      syncLifecycleMarkers();
      cleanupTerminalJob()
        .then(function () {
          return fetchDemoPlan(id);
        })
        .then(function (data) {
          sourcePlan = clonePlan(data);
          controls.loaded(data);
          destroyAllTimelines();
          dataLoading = false;
          clearBootstrapError();
          renderAll(data);
          syncLifecycleMarkers();
        })
        .catch(reportBootstrapError);
    }
  });
  updateSolveActionAvailability();
  bootstrapDemoData();

  window.addEventListener('beforeunload', destroyAllTimelines);

  function loadAndSolve() {
    if (
      solver.isRunning() ||
      solver.getLifecycleState() === 'PAUSED' ||
      !canSolve()
    )
      return;
    cleanupTerminalJob()
      .then(function (data) {
        return data || resolvePlanForSolve();
      })
      .then(function (data) {
        return solver.start(data);
      })
      .then(function () {
        syncLifecycleMarkers();
      })
      .catch(function (err) {
        console.error('Solve start failed:', err);
      });
  }

  function pauseSolve() {
    controlPending = true;
    syncLifecycleMarkers();
    solver
      .pause()
      .catch(function (err) {
        console.error(err);
      })
      .finally(function () {
        controlPending = false;
        syncLifecycleMarkers();
      });
  }

  function resumeSolve() {
    controlPending = true;
    syncLifecycleMarkers();
    solver
      .resume()
      .catch(function (err) {
        console.error(err);
      })
      .finally(function () {
        controlPending = false;
        syncLifecycleMarkers();
      });
  }

  function cancelSolve() {
    controlPending = true;
    syncLifecycleMarkers();
    solver
      .cancel()
      .catch(function (err) {
        console.error(err);
      })
      .finally(function () {
        controlPending = false;
        syncLifecycleMarkers();
      });
  }

  function openAnalysis() {
    var jobId = solver.getJobId();
    if (jobId == null || jobId === '') return;
    backend
      .analyzeSnapshot(jobId)
      .then(function (analysis) {
        lastAnalysis = analysis;
        analysisModal.setBody(IAP.analysis(analysis.analysis || analysis));
        analysisModal.open();
      })
      .catch(function () {});
  }

  function renderAll(data) {
    currentPlan = clonePlan(data);
    statusBar.updateConstraintDots([]);
    IAP.update(currentPlan);
    renderViews(currentPlan);
  }

  function resolvePlanForSolve() {
    var data = clonePlan(sourcePlan);
    data.duration_seconds = controls.seconds();
    return Promise.resolve(data);
  }

  function bootstrapDemoData() {
    fetchDemoCatalog()
      .then(function (catalog) {
        demoCatalog = catalog;
        controls.catalog(catalog);
        clearBootstrapError();
        return fetchDemoPlan(catalog.defaultId);
      })
      .then(function (data) {
        sourcePlan = clonePlan(data);
        controls.loaded(data);
        dataLoading = false;
        renderAll(data);
        syncLifecycleMarkers();
      })
      .catch(function (err) {
        reportBootstrapError(err);
      });
  }

  function fetchDemoCatalog() {
    return requestJson('/instances', 'instances').then(function (catalog) {
      if (
        !catalog ||
        typeof catalog.defaultId !== 'string' ||
        !Array.isArray(catalog.availableIds)
      ) {
        throw new Error(
          'demo data catalog is missing defaultId or availableIds'
        );
      }
      if (catalog.availableIds.indexOf(catalog.defaultId) === -1) {
        throw new Error(
          'demo data catalog defaultId is not present in availableIds'
        );
      }
      return {
        defaultId: catalog.defaultId,
        availableIds: catalog.availableIds.slice()
      };
    });
  }

  function fetchDemoPlan(demoId) {
    return requestJson(
      '/instances/' + encodeURIComponent(demoId),
      'instance "' + demoId + '"'
    );
  }

  function requestJson(path, label) {
    return fetch(path).then(function (response) {
      if (!response.ok) {
        throw new Error(label + ' returned HTTP ' + response.status);
      }
      return response.json();
    });
  }

  function canSolve() {
    return (
      !dataLoading &&
      !bootstrapError &&
      !!sourcePlan &&
      !!demoCatalog.defaultId &&
      (!controls || controls.valid())
    );
  }

  function reportBootstrapError(err) {
    bootstrapError = describeError(err);
    bootstrapNotice.textContent =
      'Demo data bootstrap failed: ' + bootstrapError;
    bootstrapNotice.style.display = '';
    app.dataset.bootstrapError = 'true';
    updateSolveActionAvailability();
    console.error('Demo data bootstrap failed:', err);
  }

  function clearBootstrapError() {
    bootstrapError = null;
    bootstrapNotice.textContent = '';
    bootstrapNotice.style.display = 'none';
    delete app.dataset.bootstrapError;
  }

  function describeError(err) {
    if (err && err.message) {
      return err.message;
    }
    return String(err || 'unknown error');
  }

  function updateSolveActionAvailability() {
    var solveButton = findHeaderButton('Solve');
    var disabled =
      !canSolve() ||
      solver.isRunning() ||
      solver.getLifecycleState() === 'PAUSED';
    if (!solveButton) return;
    solveButton.disabled = disabled;
    solveButton.setAttribute('aria-disabled', disabled ? 'true' : 'false');
    solveButton.title = disabled
      ? bootstrapError
        ? 'Demo data bootstrap failed.'
        : 'Loading demo data catalog...'
      : '';
  }

  function findHeaderButton(label) {
    var buttons = header.querySelectorAll('button');
    for (var i = 0; i < buttons.length; i += 1) {
      var text = (buttons[i].textContent || '').trim();
      if (text === label) {
        return buttons[i];
      }
    }
    return null;
  }

  function cleanupTerminalJob() {
    var state = solver.getLifecycleState();
    var jobId = solver.getJobId();
    if (
      jobId == null ||
      jobId === '' ||
      state === 'IDLE' ||
      state === 'PAUSED' ||
      solver.isRunning()
    ) {
      return Promise.resolve(null);
    }
    return solver
      .delete()
      .then(function () {
        lastAnalysis = null;
        syncLifecycleMarkers();
        return null;
      })
      .catch(function (err) {
        console.error('Delete failed:', err);
        throw err;
      });
  }

  function syncLifecycleMarkers(meta) {
    var jobId = solver.getJobId();
    var snapshotRevision = solver.getSnapshotRevision();
    var lifecycleState =
      meta && meta.lifecycleState
        ? meta.lifecycleState
        : solver.getLifecycleState();

    if (jobId) {
      app.dataset.jobId = String(jobId);
    } else {
      delete app.dataset.jobId;
    }
    if (snapshotRevision != null) {
      app.dataset.snapshotRevision = String(snapshotRevision);
    } else {
      delete app.dataset.snapshotRevision;
    }
    if (lifecycleState && lifecycleState !== 'IDLE') {
      app.dataset.lifecycleState = lifecycleState;
    } else {
      delete app.dataset.lifecycleState;
    }
    updateSolveActionAvailability();
    if (controls) controls.sync(solver, dataLoading);
    header.sfControls.pauseBtn.disabled =
      controlPending || !currentPlan || !currentPlan.score;
    header.sfControls.resumeBtn.disabled = controlPending;
    header.sfControls.cancelBtn.disabled =
      controlPending || !currentPlan || !currentPlan.score;
  }

  function clonePlan(data) {
    return JSON.parse(JSON.stringify(data));
  }

  function updateConstraintAnalysis(meta) {
    if (!meta || !meta.jobId) return;
    backend
      .analyzeSnapshot(meta.jobId)
      .then(function (result) {
        if (
          !lastMeta ||
          lastMeta.jobId !== meta.jobId ||
          lastMeta.snapshotRevision !== meta.snapshotRevision ||
          solver.isRunning()
        )
          return;
        var analysis = result.analysis || result;
        statusBar.colorDotsFromAnalysis(
          analysis.constraints.map(function (rule) {
            return {
              name: rule.name,
              score: rule.score,
              type:
                rule.name === 'contacts' || rule.name.indexOf('contact_') === 0
                  ? 'soft'
                  : 'hard'
            };
          })
        );
      })
      .catch(function (err) {
        SF.showError(err.message || 'Score analysis failed');
      });
  }

  function renderViews(data) {
    (uiModel.views || []).forEach(function (view) {
      if (view.id !== activeTab) return;
      var container = document.getElementById('view-' + view.id);
      if (!container) return;
      renderTimelinePanel(container, view.id, IAP.payload(data, view));
    });
  }

  function renderTimelinePanel(container, viewId, payload, emptyMessage) {
    container.innerHTML = '';
    if (!payload) {
      destroyTimeline(viewId);
      container.appendChild(SF.el('p', null, emptyMessage));
      return;
    }

    container.appendChild(ensureTimeline(viewId, payload.timeline).el);
  }

  function ensureTimeline(viewId, timelineConfig) {
    var timeline = viewTimelines[viewId];
    if (!timeline) {
      timeline = SF.rail.createTimeline(timelineConfig);
      if (sharedViewport) timeline.setViewport(sharedViewport);
      viewTimelines[viewId] = timeline;
      return timeline;
    }

    timeline.setModel(timelineConfig.model);
    return timeline;
  }

  function destroyTimeline(viewId) {
    var timeline = viewTimelines[viewId];
    if (!timeline) return;
    timeline.destroy();
    delete viewTimelines[viewId];
  }

  function destroyAllTimelines() {
    sharedViewport = null;
    Object.keys(viewTimelines).forEach(function (viewId) {
      destroyTimeline(viewId);
    });
  }

  function factLabel(fact, fallback) {
    if (!fact) return String(fallback);
    return fact.name || fact.id || fallback;
  }

  function entityLabel(entity, fallback) {
    if (!entity) return String(fallback);
    return entity.name || entity.id || fallback;
  }

  function title(text) {
    return String(text || '')
      .replace(/_/g, ' ')
      .replace(/\b\w/g, function (match) {
        return match.toUpperCase();
      });
  }
})();
