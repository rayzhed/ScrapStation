import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { GameCard, CacheEntry } from '$lib/types';

interface GamesState {
    all: GameCard[];
    serverPage: number;
    isSearchResult: boolean;
}

const initialState: GamesState = {
    all: [],
    serverPage: 1,
    isSearchResult: false,
};

export const gamesState = writable<GamesState>(initialState);
export const loading = writable(false);
export const error = writable('');
export const loadingSource = writable<string>('');

// All games on the current server page (no client-side slicing)
export const games = derived(gamesState, $state => $state.all);
export const currentPage = derived(gamesState, $state => $state.serverPage);
export const isSearchResult = derived(gamesState, $state => $state.isSearchResult);
// "Next" is available as long as a full page came back (30 = typical page size)
export const hasNextPage = derived(gamesState, $state =>
    !$state.isSearchResult && $state.all.length > 0
);
export const hasPrevPage = derived(gamesState, $state =>
    !$state.isSearchResult && $state.serverPage > 1
);

// ── Cache ────────────────────────────────────────────────────────────────────

interface Cache {
    [sourceId: string]: {
        pages: Record<number, CacheEntry<GameCard[]>>;
        searches: Map<string, CacheEntry<GameCard[]>>;
    };
}

const cache: Cache = {};
const CACHE_DURATION = 5 * 60 * 1000;

function isCacheValid(timestamp: number): boolean {
    return Date.now() - timestamp < CACHE_DURATION;
}

function getCachedPage(sourceId: string, page: number): GameCard[] | null {
    const entry = cache[sourceId]?.pages[page];
    return entry && isCacheValid(entry.timestamp) ? entry.data : null;
}

function getCachedSearch(sourceId: string, query: string): GameCard[] | null {
    const entry = cache[sourceId]?.searches.get(query);
    return entry && isCacheValid(entry.timestamp) ? entry.data : null;
}

function ensureCache(sourceId: string) {
    if (!cache[sourceId]) {
        cache[sourceId] = { pages: {}, searches: new Map() };
    }
}

function setCachedPage(sourceId: string, page: number, data: GameCard[]) {
    ensureCache(sourceId);
    cache[sourceId].pages[page] = { data, timestamp: Date.now() };
}

function setCachedSearch(sourceId: string, query: string, data: GameCard[]) {
    ensureCache(sourceId);
    cache[sourceId].searches.set(query, { data, timestamp: Date.now() });
}

// ── Load page ────────────────────────────────────────────────────────────────

export async function loadGames(sourceId: string, page: number = 1) {
    const cached = getCachedPage(sourceId, page);
    if (cached) {
        loadingSource.set(sourceId);
        gamesState.set({ all: cached, serverPage: page, isSearchResult: false });
        error.set('');
        return;
    }

    loading.set(true);
    loadingSource.set(sourceId);
    error.set('');

    try {
        const result = await invoke<GameCard[]>('load_dynamic_source', { sourceId, page });

        setCachedPage(sourceId, page, result);
        gamesState.set({ all: result, serverPage: page, isSearchResult: false });
        error.set('');
    } catch (e) {
        error.set(String(e));
        gamesState.set({ all: [], serverPage: page, isSearchResult: false });
    } finally {
        loading.set(false);
    }
}

// ── Search ───────────────────────────────────────────────────────────────────

export async function searchGames(sourceId: string, query: string) {
    const cached = getCachedSearch(sourceId, query);
    if (cached) {
        loadingSource.set(sourceId);
        gamesState.set({ all: cached, serverPage: 1, isSearchResult: true });
        error.set('');
        return;
    }

    loading.set(true);
    loadingSource.set(sourceId);
    error.set('');

    try {
        const result = await invoke<GameCard[]>('search_dynamic_source', { sourceId, query });

        setCachedSearch(sourceId, query, result);
        gamesState.set({ all: result, serverPage: 1, isSearchResult: true });
        error.set('');
    } catch (e) {
        error.set(String(e));
        gamesState.set({ all: [], serverPage: 1, isSearchResult: true });
    } finally {
        loading.set(false);
    }
}

// ── Pagination ───────────────────────────────────────────────────────────────

export async function goToNextPage(sourceId: string) {
    const page = get(gamesState).serverPage;
    await loadGames(sourceId, page + 1);
    window.scrollTo({ top: 0, behavior: 'smooth' });
}

export async function goToPrevPage(sourceId: string) {
    const page = get(gamesState).serverPage;
    if (page > 1) {
        await loadGames(sourceId, page - 1);
        window.scrollTo({ top: 0, behavior: 'smooth' });
    }
}
