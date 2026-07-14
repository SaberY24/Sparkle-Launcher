import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ChangelogVersion {
  id: string;
  releasedAt: number;
  content: string;
}

interface ChangelogSectionBlock {
  type: "section";
  heading: string;
  items: string[];
}

interface ChangelogTitleBlock {
  type: "title";
  text: string;
}

export type ChangelogBlock = ChangelogTitleBlock | ChangelogSectionBlock;

// Module-scope (singleton): survives ChangelogTab.vue unmount/remount when
// switching tabs, so we don't refetch every time the user revisits the tab.
const changelogs = ref<ChangelogVersion[]>([]);
const changelogsLoading = ref(false);
const changelogsRefreshing = ref(false);
const changelogsError = ref("");
const expandedChangelog = ref<string | null>(null);
let changelogsLoadedOnce = false;

async function loadChangelogs(forceRefresh = false) {
  changelogsLoading.value = true;
  changelogsError.value = "";
  try {
    changelogs.value = await invoke<ChangelogVersion[]>("fetch_changelogs", { forceRefresh });
    changelogsLoadedOnce = true;
    if (changelogs.value.length > 0 && !expandedChangelog.value) {
      expandedChangelog.value = changelogs.value[0].id;
    }
  } catch (e: any) {
    changelogsError.value = e?.message || "Failed to load the changelog.";
  } finally {
    changelogsLoading.value = false;
  }
}

async function refreshChangelogs() {
  if (changelogsLoading.value || changelogsRefreshing.value) return;
  changelogsRefreshing.value = true;
  try {
    await loadChangelogs(true);
  } finally {
    changelogsRefreshing.value = false;
  }
}

function toggleChangelog(id: string) {
  expandedChangelog.value = expandedChangelog.value === id ? null : id;
}

function formatChangelogDate(timestampMs: number): string {
  return new Date(timestampMs).toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function inlineMarkdown(text: string): string {
  const escapeHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  let out = escapeHtml(text);
  out = out.replace(/`([^`]+)`/g, "<code>$1</code>");
  out = out.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  out = out.replace(/(^|[^*])\*([^*]+)\*(?!\*)/g, "$1<em>$2</em>");
  return out;
}

function parseChangelog(markdown: string): ChangelogBlock[] {
  const blocks: ChangelogBlock[] = [];
  let current: ChangelogSectionBlock | null = null;

  for (const raw of markdown.split("\n")) {
    const line = raw.trim();
    if (!line) continue;

    const h1 = line.match(/^#\s+(.*)$/);
    const h2 = line.match(/^##\s+(.*)$/);
    const li = line.match(/^-\s+(.*)$/);

    if (h1) {
      blocks.push({ type: "title", text: h1[1] });
      current = null;
      continue;
    }
    if (h2) {
      current = { type: "section", heading: h2[1], items: [] };
      blocks.push(current);
      continue;
    }
    if (li) {
      if (!current) {
        current = { type: "section", heading: "", items: [] };
        blocks.push(current);
      }
      current.items.push(li[1]);
    }
  }

  return blocks;
}

function sectionCategory(heading: string): "add" | "remove" | "update" | "change" | "default" {
  const h = heading.toLowerCase();
  if (h.includes("remov")) return "remove";
  if (h.includes("add")) return "add";
  if (h.includes("updat")) return "update";
  if (h.includes("chang")) return "change";
  return "default";
}

export function useChangelogs() {
  return {
    changelogs,
    changelogsLoading,
    changelogsRefreshing,
    changelogsError,
    expandedChangelog,
    isLoadedOnce: () => changelogsLoadedOnce,
    loadChangelogs,
    refreshChangelogs,
    toggleChangelog,
    formatChangelogDate,
    inlineMarkdown,
    parseChangelog,
    sectionCategory,
  };
}