// GENERATED from crates/central-web/ts/security.ts via TypeScript 7.1 — do not edit directly.
// Progressive enhancement for /security:
// - Fetches /static/audio/security/{lang}/cues.json when available.
// - Synchronizes audio playback time with each diagram's data-step and radio switch.
// - Falls back smoothly to an interactive visual step tour when audio files have not been generated yet.
// Plain DOM only (Trusted Types safe, no markup strings, no console output).
const AUDIO_SRC_PATTERN = /^\/static\/audio\/security\/[a-z-]+\/[a-z]+\.mp3$/;
const TOUR_STEP_MS = 3500;
let activeCleanup = null;
let sharedMuted = false;
function isRecord(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function parseStepCue(raw) {
    if (!isRecord(raw))
        return null;
    const step = raw.step;
    const start = raw.start;
    const end = raw.end;
    if (typeof step !== "number" || !Number.isInteger(step) || step < 1 || step > 5)
        return null;
    if (typeof start !== "number" || !Number.isFinite(start) || start < 0)
        return null;
    if (typeof end !== "number" || !Number.isFinite(end) || end < start)
        return null;
    return { step, start, end };
}
function parsePartCues(raw) {
    if (!isRecord(raw))
        return null;
    const src = raw.src;
    const duration = raw.duration;
    const stepsRaw = raw.steps;
    if (typeof src !== "string" || !AUDIO_SRC_PATTERN.test(src))
        return null;
    if (typeof duration !== "number" || !Number.isFinite(duration) || duration <= 0)
        return null;
    if (!Array.isArray(stepsRaw))
        return null;
    const steps = [];
    for (const item of stepsRaw) {
        const cue = parseStepCue(item);
        if (!cue)
            return null;
        steps.push(cue);
    }
    return { src, duration, steps };
}
function parseCuesManifest(raw) {
    if (!isRecord(raw) || raw.available !== true || !isRecord(raw.parts))
        return null;
    const parts = {};
    for (const [key, value] of Object.entries(raw.parts)) {
        const parsed = parsePartCues(value);
        if (parsed)
            parts[key] = parsed;
    }
    if (Object.keys(parts).length === 0)
        return null;
    return { available: true, parts };
}
function formatClock(seconds) {
    const safe = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
    const mins = Math.floor(safe / 60);
    const secs = safe % 60;
    return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
}
function formatStepLabel(bar, step, total) {
    if (step <= 0)
        return bar.dataset.ready ?? "";
    const template = bar.dataset.stepFmt ?? "{n} / {total}";
    return template.replaceAll("{n}", String(step)).replaceAll("{total}", String(total));
}
function findCueStep(steps, seconds) {
    if (steps.length === 0)
        return 0;
    for (const cue of steps) {
        if (seconds >= cue.start && seconds < cue.end)
            return cue.step;
    }
    const first = steps[0];
    if (first && seconds < first.start)
        return first.step;
    const last = steps[steps.length - 1];
    return last ? last.step : 0;
}
function getSelectedRadioStep(fig) {
    if (!fig)
        return 0;
    for (let n = 1; n <= 5; n += 1) {
        const radio = fig.querySelector(`input.sec-p${n}`);
        if (radio?.checked)
            return n;
    }
    return 0;
}
function setRadioStep(fig, part, step) {
    if (!fig)
        return;
    const target = document.getElementById(`sec-${part}-${step}`);
    if (target instanceof HTMLInputElement)
        target.checked = true;
    if (step >= 1 && step <= 5)
        fig.dataset.step = String(step);
    else
        delete fig.dataset.step;
}
function syncBarVisuals(state) {
    const { bar, playBtn, playLabel, muteBtn, statusEl, partCues, playing, currentStep, total } = state;
    if (playing)
        bar.dataset.playing = "true";
    else
        delete bar.dataset.playing;
    playBtn.setAttribute("aria-pressed", String(playing));
    if (playLabel) {
        if (partCues) {
            playLabel.textContent = playing ? (bar.dataset.pause ?? "") : (bar.dataset.play ?? "");
        }
        else {
            playLabel.textContent = playing ? (bar.dataset.tourPause ?? "") : (bar.dataset.tourStart ?? "");
        }
    }
    if (muteBtn) {
        if (sharedMuted)
            bar.dataset.muted = "true";
        else
            delete bar.dataset.muted;
        muteBtn.setAttribute("aria-pressed", String(sharedMuted));
        const muteLabel = sharedMuted ? (bar.dataset.unmute ?? "") : (bar.dataset.mute ?? "");
        muteBtn.setAttribute("aria-label", muteLabel);
        muteBtn.title = muteLabel;
    }
    if (statusEl) {
        if (state.intro) {
            statusEl.textContent = currentStep > 0 ? formatStepLabel(bar, currentStep, total) : (bar.dataset.ready ?? "");
        }
        else {
            const displayStep = currentStep > 0 ? currentStep : 1;
            statusEl.textContent = formatStepLabel(bar, displayStep, total);
        }
    }
}
function clearRaf(state) {
    if (state.rafId !== null) {
        cancelAnimationFrame(state.rafId);
        state.rafId = null;
    }
}
function clearTourTimer(state) {
    if (state.tourTimer !== null) {
        window.clearInterval(state.tourTimer);
        state.tourTimer = null;
    }
}
function updateAudioReadout(state) {
    const { audio, partCues, rangeEl, timeEl } = state;
    if (!partCues)
        return;
    const rawDuration = audio && Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : partCues.duration;
    const current = audio ? audio.currentTime : 0;
    if (rangeEl && rawDuration > 0) {
        const scaled = Math.min(1000, Math.max(0, Math.round((current / rawDuration) * 1000)));
        rangeEl.value = String(scaled);
    }
    if (timeEl) {
        timeEl.textContent = `${formatClock(current)} / ${formatClock(rawDuration)}`;
    }
    if (state.playing && partCues.steps.length > 0) {
        const cueStep = findCueStep(partCues.steps, current);
        if (cueStep > 0 && cueStep !== state.currentStep) {
            state.currentStep = cueStep;
            setRadioStep(state.fig, state.part, cueStep);
            syncBarVisuals(state);
        }
    }
}
function stopPlayer(state, resetToAutoLoop) {
    clearTourTimer(state);
    clearRaf(state);
    if (state.audio && !state.audio.paused) {
        state.audio.pause();
    }
    state.playing = false;
    if (resetToAutoLoop) {
        state.currentStep = 0;
        setRadioStep(state.fig, state.part, 0);
        if (state.audio)
            state.audio.currentTime = 0;
        updateAudioReadout(state);
    }
    syncBarVisuals(state);
}
function stopAllExcept(players, active) {
    for (const player of players) {
        if (player !== active && player.playing) {
            stopPlayer(player, false);
        }
    }
}
function ensureAudio(state, onEnded, signal) {
    if (!state.partCues)
        return null;
    if (state.audio)
        return state.audio;
    const audio = new Audio(state.partCues.src);
    audio.preload = "metadata";
    audio.muted = sharedMuted;
    audio.addEventListener("loadedmetadata", () => updateAudioReadout(state), { signal });
    audio.addEventListener("timeupdate", () => updateAudioReadout(state), { signal });
    audio.addEventListener("ended", onEnded, { signal });
    audio.addEventListener("error", () => {
        // If a clip fails to load on a static build without audio, degrade gracefully to visual tour.
        state.partCues = null;
        if (state.muteBtn)
            state.muteBtn.hidden = true;
        if (state.trackEl)
            state.trackEl.hidden = true;
        if (state.modeEl)
            state.modeEl.textContent = state.bar.dataset.visualOnly ?? "";
        stopPlayer(state, true);
    }, { signal });
    state.audio = audio;
    return audio;
}
function scheduleAudioTick(state) {
    clearRaf(state);
    const tick = () => {
        if (!state.playing || !state.audio || state.audio.paused) {
            state.rafId = null;
            return;
        }
        updateAudioReadout(state);
        state.rafId = requestAnimationFrame(tick);
    };
    state.rafId = requestAnimationFrame(tick);
}
function highlightTocFlow(step) {
    const links = document.querySelectorAll(".sec-toc li a");
    for (let idx = 0; idx < links.length; idx += 1) {
        const link = links[idx];
        if (!link)
            continue;
        if (step > 0 && idx + 1 === step)
            link.setAttribute("aria-current", "step");
        else
            link.removeAttribute("aria-current");
    }
}
function applyTourStep(state, step) {
    state.currentStep = step;
    if (state.intro) {
        highlightTocFlow(step);
    }
    else {
        setRadioStep(state.fig, state.part, step);
    }
    syncBarVisuals(state);
}
function startTourLoop(state) {
    clearTourTimer(state);
    const startStep = state.currentStep >= 1 && state.currentStep <= state.total ? state.currentStep : 1;
    state.playing = true;
    applyTourStep(state, startStep);
    state.tourTimer = window.setInterval(() => {
        if (!state.playing) {
            clearTourTimer(state);
            return;
        }
        const next = state.currentStep + 1;
        if (next > state.total) {
            if (state.intro)
                highlightTocFlow(0);
            stopPlayer(state, true);
            return;
        }
        applyTourStep(state, next);
    }, TOUR_STEP_MS);
}
function seekFlowStep(state, players, step, signal) {
    const clamped = Math.min(state.total, Math.max(1, step));
    state.currentStep = clamped;
    setRadioStep(state.fig, state.part, clamped);
    if (state.partCues) {
        const cue = state.partCues.steps.find((item) => item.step === clamped);
        if (cue && state.playing) {
            const audio = ensureAudio(state, () => {
                stopPlayer(state, true);
            }, signal);
            if (audio) {
                audio.currentTime = cue.start;
                updateAudioReadout(state);
            }
        }
    }
    else if (state.playing) {
        clearTourTimer(state);
        stopAllExcept(players, state);
        startTourLoop(state);
    }
    syncBarVisuals(state);
}
function buildPlayerState(bar) {
    const part = bar.dataset.secAudio ?? "";
    const playBtn = bar.querySelector("[data-sec-play]");
    if (!part || !playBtn)
        return null;
    const intro = part === "intro";
    const parsedTotal = Number.parseInt(bar.dataset.total ?? "5", 10);
    const total = intro ? 4 : Number.isInteger(parsedTotal) && parsedTotal > 0 ? parsedTotal : 5;
    const fig = intro ? null : document.querySelector(`[data-sec-fig="${part}"]`);
    return {
        part,
        intro,
        total,
        bar,
        fig,
        playBtn,
        playLabel: bar.querySelector("[data-sec-play-label]"),
        prevBtn: bar.querySelector("[data-sec-prev]"),
        nextBtn: bar.querySelector("[data-sec-next]"),
        muteBtn: bar.querySelector("[data-sec-mute]"),
        modeEl: bar.querySelector("[data-sec-mode]"),
        statusEl: bar.querySelector("[data-sec-status]"),
        trackEl: bar.querySelector("[data-sec-track]"),
        rangeEl: bar.querySelector("[data-sec-range]"),
        timeEl: bar.querySelector("[data-sec-time]"),
        partCues: null,
        audio: null,
        tourTimer: null,
        rafId: null,
        currentStep: getSelectedRadioStep(fig),
        playing: false,
    };
}
function wirePlayer(state, players, signal) {
    const onEnded = () => {
        if (state.intro)
            highlightTocFlow(0);
        stopPlayer(state, true);
    };
    state.playBtn.addEventListener("click", () => {
        if (state.playing) {
            stopPlayer(state, false);
            return;
        }
        stopAllExcept(players, state);
        if (state.partCues) {
            const audio = ensureAudio(state, onEnded, signal);
            if (!audio)
                return;
            audio.muted = sharedMuted;
            if (!state.intro && state.currentStep >= 1 && state.currentStep <= 5) {
                const cue = state.partCues.steps.find((item) => item.step === state.currentStep);
                if (cue && (audio.currentTime < cue.start || audio.currentTime >= cue.end)) {
                    audio.currentTime = cue.start;
                }
            }
            state.playing = true;
            if (!state.intro && state.currentStep === 0 && state.partCues.steps[0]) {
                const firstStep = state.partCues.steps[0].step;
                state.currentStep = firstStep;
                setRadioStep(state.fig, state.part, firstStep);
            }
            syncBarVisuals(state);
            audio
                .play()
                .then(() => {
                if (state.playing)
                    scheduleAudioTick(state);
            })
                .catch(() => {
                stopPlayer(state, false);
            });
        }
        else {
            startTourLoop(state);
        }
    }, { signal });
    if (state.prevBtn) {
        state.prevBtn.addEventListener("click", () => {
            const base = state.currentStep > 1 ? state.currentStep - 1 : state.total;
            seekFlowStep(state, players, base, signal);
        }, { signal });
    }
    if (state.nextBtn) {
        state.nextBtn.addEventListener("click", () => {
            const next = state.currentStep >= 1 && state.currentStep < state.total ? state.currentStep + 1 : 1;
            seekFlowStep(state, players, next, signal);
        }, { signal });
    }
    if (state.muteBtn) {
        state.muteBtn.addEventListener("click", () => {
            sharedMuted = !sharedMuted;
            for (const player of players) {
                if (player.audio)
                    player.audio.muted = sharedMuted;
                syncBarVisuals(player);
            }
        }, { signal });
    }
    if (state.rangeEl) {
        state.rangeEl.addEventListener("input", () => {
            if (!state.partCues || !state.rangeEl)
                return;
            const audio = ensureAudio(state, onEnded, signal);
            if (!audio)
                return;
            const duration = Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : state.partCues.duration;
            const ratio = Math.min(1, Math.max(0, Number(state.rangeEl.value) / 1000));
            audio.currentTime = ratio * duration;
            updateAudioReadout(state);
            if (!state.intro && state.partCues.steps.length > 0) {
                const cueStep = findCueStep(state.partCues.steps, audio.currentTime);
                if (cueStep > 0) {
                    state.currentStep = cueStep;
                    setRadioStep(state.fig, state.part, cueStep);
                    syncBarVisuals(state);
                }
            }
        }, { signal });
    }
    if (state.fig) {
        const radios = state.fig.querySelectorAll("input.sec-pick");
        for (const radio of radios) {
            radio.addEventListener("change", () => {
                const selected = getSelectedRadioStep(state.fig);
                if (selected === 0) {
                    if (state.fig)
                        delete state.fig.dataset.step;
                    stopPlayer(state, false);
                    state.currentStep = 0;
                    syncBarVisuals(state);
                    return;
                }
                seekFlowStep(state, players, selected, signal);
            }, { signal });
        }
        const items = state.fig.querySelectorAll("ol.sec-list > li[data-sec-step]");
        for (const item of items) {
            item.addEventListener("click", (event) => {
                const target = event.target instanceof Element ? event.target : null;
                if (target?.closest("a, button, input, label, code"))
                    return;
                const selection = window.getSelection()?.toString() ?? "";
                if (selection.length > 0)
                    return;
                const stepNum = Number.parseInt(item.dataset.secStep ?? "", 10);
                if (Number.isInteger(stepNum) && stepNum >= 1 && stepNum <= 5) {
                    seekFlowStep(state, players, stepNum, signal);
                }
            }, { signal });
        }
    }
}
async function loadCues(lang, signal) {
    if (!lang || !/^[a-z-]+$/.test(lang))
        return null;
    try {
        const response = await fetch(`/static/audio/security/${encodeURIComponent(lang)}/cues.json`, {
            signal,
        });
        if (!response.ok)
            return null;
        const data = await response.json();
        return parseCuesManifest(data);
    }
    catch {
        return null;
    }
}
function initSecurityPage() {
    if (activeCleanup) {
        activeCleanup();
        activeCleanup = null;
    }
    const bars = [...document.querySelectorAll("[data-sec-audio]")];
    if (bars.length === 0)
        return;
    const leaving = new AbortController();
    const introCard = document.querySelector(".sec-intro-card");
    if (introCard)
        introCard.hidden = false;
    const players = [];
    for (const bar of bars) {
        bar.hidden = false;
        const state = buildPlayerState(bar);
        if (state)
            players.push(state);
    }
    for (const player of players) {
        wirePlayer(player, players, leaving.signal);
        syncBarVisuals(player);
    }
    const cleanup = () => {
        for (const player of players) {
            stopPlayer(player, false);
        }
        highlightTocFlow(0);
        leaving.abort();
    };
    activeCleanup = cleanup;
    document.addEventListener("baylee:leave", cleanup, { once: true, signal: leaving.signal });
    const lang = bars[0]?.dataset.lang ?? document.documentElement.lang.toLowerCase();
    void loadCues(lang, leaving.signal).then((manifest) => {
        if (leaving.signal.aborted || !manifest)
            return;
        for (const player of players) {
            const partData = manifest.parts[player.part];
            if (!partData)
                continue;
            player.partCues = partData;
            if (player.modeEl)
                player.modeEl.textContent = player.bar.dataset.narrated ?? "";
            if (player.muteBtn)
                player.muteBtn.hidden = false;
            if (player.trackEl)
                player.trackEl.hidden = false;
            updateAudioReadout(player);
            syncBarVisuals(player);
        }
    });
}
if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initSecurityPage, { once: true });
}
else {
    initSecurityPage();
}
document.addEventListener("baylee:page", initSecurityPage);
export {};
