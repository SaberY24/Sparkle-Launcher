<script setup lang="ts">
import { onMounted } from "vue";
import EmptyState from "../ui/EmptyState.vue";
import { useOptimizedScroll } from "../../composables/useOptimizedScroll";
import { useChangelogs } from "../../composables/useChangelogs";

const { onScroll: onChangelogScroll } = useOptimizedScroll({
  frameSyncInterval: 3,
  settleDelay: 150,
  minScrollDelta: 0.5,
});

const {
  changelogs,
  changelogsLoading,
  changelogsRefreshing,
  changelogsError,
  expandedChangelog,
  isLoadedOnce,
  loadChangelogs,
  refreshChangelogs,
  toggleChangelog,
  formatChangelogDate,
  inlineMarkdown,
  parseChangelog,
  sectionCategory,
} = useChangelogs();

// Mirrors the old `watch(activeTab, tab => { if (tab === 'changelog' &&
// !changelogsLoadedOnce) loadChangelogs() })`: HomeView still mounts this
// component only while the Changelog tab is active, so onMounted fires at
// the same point the old watcher did — but only fetches once thanks to the
// singleton `changelogsLoadedOnce` flag inside the composable.
onMounted(() => {
  if (!isLoadedOnce()) {
    loadChangelogs();
  }
});
</script>

<template>
  <main class="changelog-tab">
    <div class="changelog-panel">
      <div class="changelog-toolbar">
        <div class="changelog-toolbar-title">
          <span class="changelog-toolbar-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
              <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
              <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
            </svg>
          </span>
          <span>Changelog</span>
          <span v-if="changelogs.length > 0" class="ct-tab-count">{{ changelogs.length }}</span>
        </div>
        <button
          type="button"
          class="mods-refresh-btn"
          :disabled="changelogsLoading || changelogsRefreshing"
          v-tooltip="'Refresh changelog'"
          @click="refreshChangelogs"
        >
          <svg
            :class="{ spin: changelogsLoading || changelogsRefreshing }"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            width="14"
            height="14"
          >
            <polyline points="23 4 23 10 17 10" />
            <polyline points="1 20 1 14 7 14" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
        </button>
      </div>

      <div v-if="changelogsError" class="mods-error-banner">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <span>{{ changelogsError }}</span>
      </div>

      <div class="changelog-scroller" @scroll="onChangelogScroll">
        <EmptyState v-if="changelogsLoading && changelogs.length === 0" icon="spinner" message="Loading changelog..." />

        <EmptyState
          v-else-if="changelogs.length === 0"
          icon="book"
          message="No changelog entries available yet."
          action-label="Refresh"
          @action="refreshChangelogs"
        />

        <div v-else class="changelog-list">
          <div
            v-for="(v, idx) in changelogs"
            :key="v.id"
            class="changelog-item"
            :class="{ expanded: expandedChangelog === v.id }"
            v-once
          >
            <button class="changelog-item-header" @click="toggleChangelog(v.id)">
              <div class="changelog-item-heading">
                <span class="changelog-version-badge">{{ v.id }}</span>
                <span v-if="idx === 0" class="changelog-latest-tag">Latest</span>
                <span class="changelog-date">{{ formatChangelogDate(v.releasedAt) }}</span>
              </div>
              <svg
                class="log-dropdown-chevron"
                :class="{ rotated: expandedChangelog === v.id }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                width="14"
                height="14"
              >
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </button>

            <transition name="changelog-body">
              <div v-if="expandedChangelog === v.id" class="changelog-item-body" v-once>
                <template v-for="(block, bIdx) in parseChangelog(v.content)" :key="bIdx">
                  <h3 v-if="block.type === 'title'" class="changelog-title">{{ block.text }}</h3>
                  <div v-else class="changelog-section" :class="`cat-${sectionCategory(block.heading)}`">
                    <div v-if="block.heading" class="changelog-section-header">
                      <span class="changelog-section-icon">
                        <svg v-if="sectionCategory(block.heading) === 'add'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" width="12" height="12">
                          <line x1="12" y1="5" x2="12" y2="19" />
                          <line x1="5" y1="12" x2="19" y2="12" />
                        </svg>
                        <svg v-else-if="sectionCategory(block.heading) === 'remove'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" width="12" height="12">
                          <line x1="5" y1="12" x2="19" y2="12" />
                        </svg>
                        <svg v-else-if="sectionCategory(block.heading) === 'update'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" width="12" height="12">
                          <polyline points="23 4 23 10 17 10" />
                          <polyline points="1 20 1 14 7 14" />
                          <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
                        </svg>
                        <span v-else class="changelog-section-bullet"></span>
                      </span>
                      {{ block.heading }}
                    </div>
                    <ul class="changelog-section-list">
                      <li v-for="(item, iIdx) in block.items" :key="iIdx">
                        <span class="changelog-li-dot"></span>
                        <span v-html="inlineMarkdown(item)"></span>
                      </li>
                    </ul>
                  </div>
                </template>
                <p v-if="!v.content.trim()" class="changelog-empty">No notes for this version yet.</p>
              </div>
            </transition>
          </div>
        </div>
      </div>
    </div>
  </main>
</template>