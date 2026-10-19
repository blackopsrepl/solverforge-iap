/* IAP presentation adapters for the solverforge-cli web shell. */
(function (root) {
  'use strict';
  var $ = SF.el,
    STEP = 60,
    palette = ['blue', 'violet', 'cyan', 'emerald', 'rose', 'amber', 'slate'];
  var references = new Map(),
    variant = 'cm',
    lastPlan = null,
    selected = 'exams',
    controls = null;
  var views = [
    { id: 'exams', label: 'Exams', kind: 'iap' },
    { id: 'people', label: 'Invigilators', kind: 'iap' },
    { id: 'contacts', label: 'Contacts', kind: 'iap' }
  ];
  function project(plan) {
    var people = plan.invigilators.map(function (p, i) {
        return Object.assign({}, p, {
          arrayIndex: i,
          assigned: [],
          contacts: new Set()
        });
      }),
      exams = plan.exams.map(function (e) {
        return Object.assign({}, e, { seats: [], people: [] });
      }),
      byExam = new Map(
        exams.map(function (e) {
          return [e.index, e];
        })
      ),
      occupied = new Map(),
      pairs = new Map();
    plan.seats.forEach(function (seat) {
      var exam = byExam.get(seat.exam_index);
      if (!exam) return;
      exam.seats.push(seat);
      var person =
        seat.invigilator_idx == null ? null : people[seat.invigilator_idx];
      if (!person) return;
      exam.people.push(person);
      person.assigned.push(exam);
      var key = person.arrayIndex + ':' + exam.slot;
      occupied.set(key, (occupied.get(key) || 0) + 1);
    });
    exams
      .slice()
      .sort(function (a, b) {
        return a.slot - b.slot || a.index - b.index;
      })
      .forEach(function (exam) {
        var unique = Array.from(
          new Map(
            exam.people.map(function (p) {
              return [p.arrayIndex, p];
            })
          ).values()
        );
        for (var i = 0; i < unique.length; i++)
          for (var j = i + 1; j < unique.length; j++) {
            var a = unique[i],
              b = unique[j],
              ids = [
                Math.min(a.arrayIndex, b.arrayIndex),
                Math.max(a.arrayIndex, b.arrayIndex)
              ],
              key = ids.join(':'),
              pair = pairs.get(key);
            if (!pair) {
              pair = {
                id: key,
                a: people[ids[0]],
                b: people[ids[1]],
                exams: [],
                cross: a.group !== b.group
              };
              pairs.set(key, pair);
            }
            pair.exams.push(exam);
            a.contacts.add(b.arrayIndex);
            b.contacts.add(a.arrayIndex);
          }
      });
    return {
      plan: plan,
      people: people,
      exams: exams,
      occupied: occupied,
      pairs: Array.from(pairs.values()),
      groups: Array.from(
        new Set(
          people.map(function (p) {
            return p.group;
          })
        )
      ).sort(function (a, b) {
        return a - b;
      }),
      slots: Math.max(
        1,
        ...exams.map(function (e) {
          return e.slot + 1;
        })
      )
    };
  }
  function time(slot) {
    return { startMinute: slot * STEP, endMinute: (slot + 1) * STEP };
  }
  function axis(d) {
    return {
      startMinute: 0,
      endMinute: d.slots * STEP,
      days: [
        {
          startMinute: 0,
          endMinute: d.slots * STEP,
          label: 'Slots',
          isWeekend: false
        }
      ],
      ticks: Array.from({ length: d.slots }, function (_, i) {
        return { minute: i * STEP, label: String(i) };
      }),
      initialViewport: {
        startMinute: 0,
        endMinute: Math.min(d.slots, 8) * STEP
      }
    };
  }
  function department(g) {
    return 'Dept. ' + g;
  }
  function conflict(p, e, d) {
    return (
      !p.available_slots.includes(e.slot) ||
      d.occupied.get(p.arrayIndex + ':' + e.slot) > 1
    );
  }
  function unavailable(p, d) {
    var spans = [],
      allowed = new Set(p.available_slots);
    for (var slot = 0; slot < d.slots; slot++) {
      if (allowed.has(slot)) continue;
      var tail = spans[spans.length - 1];
      if (tail && tail.endMinute === slot * STEP)
        tail.endMinute = (slot + 1) * STEP;
      else
        spans.push(
          Object.assign(time(slot), { label: 'Unavailable', tone: 'red' })
        );
    }
    return spans;
  }
  function roomModel(d, filter) {
    var rooms = Array.from(
      new Set(
        d.exams.map(function (e) {
          return e.room;
        })
      )
    ).sort(function (a, b) {
      return a.localeCompare(b, undefined, { numeric: true });
    });
    return {
      axis: axis(d),
      lanes: rooms.map(function (room) {
        var all = d.exams.filter(function (e) {
            return e.room === room;
          }),
          visible = all.filter(function (e) {
            return (
              !filter ||
              [
                e.name,
                room,
                ...e.people.map(function (p) {
                  return p.name;
                })
              ]
                .join(' ')
                .toLowerCase()
                .includes(filter)
            );
          });
        return {
          id: 'room-' + room,
          label: 'Room ' + room,
          mode: 'detailed',
          stats: [
            {
              label: 'Assigned',
              value:
                all.reduce(function (n, e) {
                  return n + e.people.length;
                }, 0) +
                ' / ' +
                all.reduce(function (n, e) {
                  return n + e.required;
                }, 0)
            }
          ],
          items: visible.map(function (e) {
            return Object.assign(time(e.slot), {
              id: e.id,
              label: e.name,
              tone: e.people.some(function (p) {
                return conflict(p, e, d);
              })
                ? 'red'
                : e.people.length === e.required
                  ? 'emerald'
                  : 'amber',
              meta: [
                {
                  label: 'Assigned',
                  value: e.people.length + ' / ' + e.required
                },
                {
                  label: 'Team',
                  value:
                    e.people
                      .map(function (p) {
                        return p.name + ' · ' + department(p.group);
                      })
                      .join('; ') || 'Unassigned'
                },
                { label: 'Slot', value: String(e.slot) }
              ]
            });
          })
        };
      })
    };
  }
  function personModel(d, filter, group) {
    var open = d.exams.flatMap(function (e) {
      return e.seats
        .filter(function (seat) {
          return seat.invigilator_idx == null;
        })
        .map(function (seat) {
          return Object.assign(time(e.slot), {
            id: seat.id,
            label: e.name,
            meta: 'Room ' + e.room + ' · position ' + (seat.position + 1),
            tone: 'red'
          });
        });
    });
    var lanes = d.people
      .filter(function (p) {
        return (
          (group == null || p.group === group) &&
          (!filter || p.name.toLowerCase().includes(filter))
        );
      })
      .map(function (p) {
        return {
          id: p.id,
          label: p.name,
          mode: 'detailed',
          badges: [department(p.group)],
          stats: [
            {
              label: 'Workload',
              value:
                p.assigned.length +
                ' / ' +
                p.min_workload +
                '–' +
                p.max_workload
            },
            { label: 'Contacts', value: p.contacts.size }
          ],
          overlays: unavailable(p, d),
          items: p.assigned.map(function (e, i) {
            return Object.assign(time(e.slot), {
              id: p.id + '-' + e.id + '-' + i,
              label: e.name,
              tone: conflict(p, e, d)
                ? 'red'
                : palette[p.group % palette.length],
              meta: [
                { label: 'Room', value: e.room },
                {
                  label: 'Team',
                  value:
                    e.people
                      .filter(function (other) {
                        return other.arrayIndex !== p.arrayIndex;
                      })
                      .map(function (other) {
                        return other.name + ' · ' + department(other.group);
                      })
                      .join('; ') || 'Solo'
                },
                { label: 'Slot', value: String(e.slot) }
              ]
            });
          })
        };
      });
    lanes.unshift({
      id: 'unassigned',
      label: 'Unassigned',
      mode: 'overview',
      badges: [String(open.length)],
      items: open
    });
    return { axis: axis(d), lanes: lanes };
  }
  function contactModel(d, filter, group, detail) {
    var cells = new Map();
    d.pairs.forEach(function (pair) {
      if (variant === 'cm' && !pair.cross) return;
      var a = Math.min(pair.a.group, pair.b.group),
        b = Math.max(pair.a.group, pair.b.group);
      if (group != null && a !== group && b !== group) return;
      pair.exams.forEach(function (e, index) {
        if (
          filter &&
          ![pair.a.name, pair.b.name, e.name]
            .join(' ')
            .toLowerCase()
            .includes(filter)
        )
          return;
        var key = a + ':' + b + ':' + e.slot,
          cell = cells.get(key);
        if (!cell) {
          cell = { a: a, b: b, slot: e.slot, fresh: 0, repeat: 0, items: [] };
          cells.set(key, cell);
        }
        if (index === 0) cell.fresh++;
        else cell.repeat++;
        cell.items.push(
          Object.assign(time(e.slot), {
            id: 'pair-' + pair.id + '-' + e.id,
            label: pair.a.name + ' ↔ ' + pair.b.name,
            tone: index === 0 ? 'emerald' : 'amber',
            meta: [
              {
                label: 'Contact',
                value: index === 0 ? 'First meeting' : 'Repeat'
              },
              { label: 'Exam', value: e.name },
              { label: 'Room', value: e.room }
            ]
          })
        );
      });
    });
    var lanes = [];
    d.groups.forEach(function (a, i) {
      d.groups.slice(i).forEach(function (b) {
        if (variant === 'cm' && a === b) return;
        if (group != null && a !== group && b !== group) return;
        var row = Array.from(cells.values())
          .filter(function (c) {
            return c.a === a && c.b === b;
          })
          .sort(function (x, y) {
            return x.slot - y.slot;
          });
        lanes.push({
          id: 'pair-' + a + '-' + b,
          label: department(a) + ' ↔ ' + b,
          mode: detail ? 'overview' : 'detailed',
          stats: [
            {
              label: 'New',
              value: row.reduce(function (n, c) {
                return n + c.fresh;
              }, 0)
            },
            {
              label: 'Repeat',
              value: row.reduce(function (n, c) {
                return n + c.repeat;
              }, 0)
            }
          ],
          items: row.map(function (c) {
            var item = Object.assign(time(c.slot), {
              id: 'cell-' + a + '-' + b + '-' + c.slot,
              label: c.fresh + ' new · ' + c.repeat + ' repeat',
              tone: c.fresh ? 'emerald' : 'amber',
              meta: [
                { label: 'Slot', value: String(c.slot) },
                { label: 'New contacts', value: String(c.fresh) },
                { label: 'Repeated meetings', value: String(c.repeat) }
              ]
            });
            if (detail) {
              item.detailItems = c.items;
              item.summary = {
                primaryLabel: c.fresh + ' new · ' + c.repeat + ' repeat',
                count: c.items.length,
                openCount: 0,
                toneSegments: [
                  { tone: 'emerald', count: c.fresh },
                  { tone: 'amber', count: c.repeat }
                ]
              };
            }
            return item;
          })
        });
      });
    });
    return { axis: axis(d), lanes: lanes };
  }
  function mount(options) {
    var search = $('input', {
        type: 'search',
        className: 'sf-gantt-view-select',
        placeholder: 'Search',
        'aria-label': 'Search',
        onInput: function () {
          options.render();
        }
      }),
      group = $('select', {
        className: 'sf-gantt-view-select',
        'aria-label': 'Department',
        onChange: function () {
          options.render();
        }
      }),
      duration = $('input', {
        type: 'number',
        min: '1',
        max: '3600',
        step: '1',
        required: true,
        value: '60',
        className: 'sf-gantt-view-select',
        'aria-label': 'Duration',
        onInput: function () {
          options.sync();
        }
      }),
      instance = $('select', {
        className: 'sf-gantt-view-select',
        'aria-label': 'Instance',
        onChange: function () {
          options.load(instance.value);
        }
      }),
      focus = $('select', {
        className: 'sf-gantt-view-select',
        'aria-label': 'Slot',
        onChange: zoom
      }),
      span = $(
        'select',
        {
          className: 'sf-gantt-view-select',
          'aria-label': 'Visible slots',
          onChange: zoom
        },
        ...[
          [8, '8 slots'],
          [16, '16 slots'],
          ['all', 'All slots']
        ].map(function (row) {
          return $('option', { value: String(row[0]) }, row[1]);
        })
      ),
      detail = $('input', {
        type: 'checkbox',
        'aria-label': 'Meeting details',
        onChange: function () {
          options.render();
        }
      });
    var summary = $('div', { className: 'iap-metrics' }),
      bar = $(
        'div',
        { className: 'iap-controls' },
        $('label', {}, 'Instance ', instance),
        $('label', {}, 'Duration ', duration),
        $('span', { className: 'sf-chip' }, 'seconds'),
        SF.createButton({
          text: '30s',
          size: 'small',
          onClick: function () {
            duration.value = '30';
            options.sync();
          }
        }),
        SF.createButton({
          text: '1m',
          size: 'small',
          onClick: function () {
            duration.value = '60';
            options.sync();
          }
        }),
        SF.createButton({
          text: '2m',
          size: 'small',
          onClick: function () {
            duration.value = '120';
            options.sync();
          }
        }),
        summary
      );
    var filters = $(
      'div',
      { className: 'iap-filters' },
      search,
      group,
      $('label', {}, 'Slot ', focus),
      span,
      SF.createButton({
        icon: 'fa-chevron-left',
        size: 'small',
        tooltip: 'Previous slots',
        onClick: function () {
          focus.value = String(
            Math.max(0, Number(focus.value) - visibleSpan())
          );
          zoom();
        }
      }),
      SF.createButton({
        icon: 'fa-chevron-right',
        size: 'small',
        tooltip: 'Next slots',
        onClick: function () {
          focus.value = String(
            Math.min(lastPlan.slots - 1, Number(focus.value) + visibleSpan())
          );
          zoom();
        }
      }),
      $('label', { className: 'iap-detail' }, detail, ' Meeting details')
    );
    options.after.after(bar, filters);
    function visibleSpan() {
      return span.value === 'all' ? lastPlan.slots : Number(span.value);
    }
    function zoom() {
      if (!lastPlan) return;
      var width = Math.min(visibleSpan(), lastPlan.slots),
        start = Math.min(Number(focus.value), lastPlan.slots - width);
      focus.value = String(start);
      options.viewport({
        startMinute: start * STEP,
        endMinute: (start + width) * STEP
      });
    }
    controls = {
      catalog: function (c) {
        instance.replaceChildren(
          ...c.availableIds.map(function (id) {
            return $(
              'option',
              { value: id },
              id.replace('LargerInput_', '').replace('Input_', '')
            );
          })
        );
        instance.value = c.defaultId;
      },
      loaded: function (plan) {
        var d = project(plan);
        group.replaceChildren(
          $('option', { value: 'all' }, 'All departments'),
          ...d.groups.map(function (g) {
            return $('option', { value: String(g) }, department(g));
          })
        );
        focus.replaceChildren(
          ...Array.from({ length: d.slots }, function (_, i) {
            return $('option', { value: String(i) }, String(i));
          })
        );
        search.value = '';
      },
      valid: function () {
        return duration.checkValidity();
      },
      seconds: function () {
        return Number(duration.value);
      },
      selected: function () {
        return instance.value;
      },
      sync: function (solver, loading) {
        var state = solver.getLifecycleState(),
          locked =
            loading ||
            solver.isRunning() ||
            ['PAUSED', 'PAUSE_REQUESTED', 'RESUMING', 'CANCELLING'].includes(
              state
            );
        instance.disabled = locked;
        duration.disabled = locked;
        bar.querySelectorAll('button').forEach(function (b) {
          b.disabled = locked;
        });
        group.hidden = selected === 'exams';
        filters.querySelector('.iap-detail').hidden = selected !== 'contacts';
      },
      filters: function () {
        return {
          search: search.value.toLowerCase(),
          group: group.value === 'all' ? null : Number(group.value),
          detail: detail.checked
        };
      },
      summary: summary,
      zoom: zoom
    };
    return controls;
  }
  function update(plan) {
    lastPlan = project(plan);
    if (!controls) return;
    var d = lastPlan,
      parts = /^(-?\d+)hard\/(-?\d+)soft$/.exec(plan.score || ''),
      ref = references.get(controls.selected()),
      count =
        variant === 'cm'
          ? d.pairs.filter(function (p) {
              return p.cross;
            }).length
          : d.pairs.length,
      filled = plan.seats.filter(function (seat) {
        return seat.invigilator_idx != null;
      }).length,
      feasible = parts && Number(parts[1]) === 0;
    var metrics = [
      filled + '/' + plan.seats.length + ' assigned',
      parts ? Math.abs(Number(parts[1])) + ' hard' : 'Not solved',
      parts ? count + ' contacts' : '— contacts'
    ];
    if (variant === 'cm' && ref) {
      metrics.push('Optimum ' + ref.optimal);
      if (feasible && count <= ref.optimal)
        metrics.push(
          ((100 * (ref.optimal - count)) / ref.optimal).toFixed(2) + '% gap'
        );
    }
    controls.summary.replaceChildren(
      ...metrics.map(function (text) {
        return $('span', { className: 'sf-chip' }, text);
      })
    );
    root.document.getElementById('sf-app').dataset.feasible =
      String(!!feasible);
  }
  function payload(plan, view) {
    selected = view.id;
    if (!lastPlan || lastPlan.plan !== plan) lastPlan = project(plan);
    var f = controls
        ? controls.filters()
        : { search: '', group: null, detail: false },
      d = lastPlan,
      model =
        view.id === 'exams'
          ? roomModel(d, f.search)
          : view.id === 'people'
            ? personModel(d, f.search, f.group)
            : contactModel(d, f.search, f.group, f.detail);
    return {
      summary: $('span'),
      timeline: {
        title: view.label,
        subtitle: ' ',
        label:
          view.id === 'exams'
            ? 'Room'
            : view.id === 'people'
              ? 'Invigilator'
              : 'Departments',
        labelWidth: view.id === 'people' ? 220 : 180,
        zoomPresets: [],
        model: model
      }
    };
  }
  function analysis(analysis) {
    var body = $('div'),
      score = analysis && analysis.score;
    body.append($('span', { className: 'sf-chip' }, score || '—'));
    ((analysis && analysis.constraints) || []).forEach(function (rule) {
      var hard =
          /hard/.test(rule.weight || '') &&
          !String(rule.weight).startsWith('0hard'),
        m = /^(-?\d+)hard/.exec(rule.score),
        valid = m && Number(m[1]) === 0;
      body.append(
        $(
          'div',
          { className: 'sf-card iap-rule' },
          $(
            'span',
            {
              className:
                'sf-badge sf-badge--' +
                (hard ? (valid ? 'low' : 'high') : 'medium')
            },
            hard ? 'Hard' : 'Soft'
          ),
          $('strong', {}, rule.name.replace(/_/g, ' ')),
          $('span', { className: 'sf-chip' }, rule.score),
          $(
            'span',
            { className: 'sf-chip' },
            String(rule.matchCount) + ' matches'
          )
        )
      );
    });
    return body;
  }
  root.IAP = {
    views: views,
    mount: mount,
    update: update,
    payload: payload,
    analysis: analysis,
    project: project,
    roomModel: roomModel,
    personModel: personModel,
    contactModel: contactModel,
    init: async function () {
      var data = await Promise.all([
        fetch('/references.json').then(function (r) {
          return r.json();
        }),
        fetch('/info').then(function (r) {
          return r.json();
        })
      ]);
      references = new Map(
        data[0].instances.map(function (r) {
          return [r.id, r];
        })
      );
      variant = data[1].variant || 'cm';
    }
  };
})(window);
