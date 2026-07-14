import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getPlayerHeadUrl, getMcHeadsUrl } from "../composables/useSkinRenderer";

export interface Account {
  id: string;
  username: string;
  uuid: string;
  access_token: string;
  account_type: string;
  refresh_token?: string;
  skin_url?: string;
  skin_texture_key?: string;
}

export const useAuthStore = defineStore("auth", () => {
  const accounts = ref<Account[]>([]);
  const activeAccountIndex = ref(0);
  const loading = ref(false);
  const error = ref("");
  const statusMessage = ref("Authenticating...");
  const renderedHeadUrl = ref<string | null>(null);
  const sessionRestored = ref(false);

  const account = computed(() => accounts.value[activeAccountIndex.value] || null);
  const isAuthenticated = computed(() => accounts.value.length > 0 && account.value !== null);
  const isPremium = computed(() => account.value?.account_type === "microsoft");
  const username = computed(() => account.value?.username ?? "Guest");
  
  const accountType = computed(() => {
    if (!account.value) return "offline";
    const rawType = account.value.account_type;
    return rawType === "microsoft" || rawType === "offline" ? rawType : "offline";
  });

  const otherAccounts = computed(() => {
    return accounts.value.filter((_, idx) => idx !== activeAccountIndex.value);
  });

  const avatarUrl = computed(() => {
    if (!account.value) return null;
    if (renderedHeadUrl.value) return renderedHeadUrl.value;
    if (isPremium.value && account.value.uuid) {
      return getMcHeadsUrl(account.value.uuid, 64);
    }
    return null;
  });

  const skinUrl = computed(() => account.value?.skin_url ?? null);
  const skinTextureKey = computed(() => account.value?.skin_texture_key ?? null);

  async function renderSkinHead() {
    if (!account.value?.skin_url) {
      renderedHeadUrl.value = null;
      return;
    }
    try {
      const url = await getPlayerHeadUrl(
        account.value.skin_url,
        account.value.skin_texture_key || account.value.uuid
      );
      renderedHeadUrl.value = url;
    } catch (err) {
      console.warn("Failed to render skin head:", err);
      if (account.value.uuid) {
        renderedHeadUrl.value = getMcHeadsUrl(account.value.uuid, 64);
      }
    }
  }

  function setActiveAccount(index: number) {
    if (index >= 0 && index < accounts.value.length) {
      activeAccountIndex.value = index;
      renderSkinHead();
      // 🔥 Guardar la sesión al cambiar de cuenta activa
      saveSession();
    }
  }

  function switchAccount(otherIndex: number) {
    const targetAccount = otherAccounts.value[otherIndex];
    if (!targetAccount) return;
    const realIndex = accounts.value.findIndex(a => a.id === targetAccount.id);
    if (realIndex !== -1) {
      setActiveAccount(realIndex);
    }
  }

  function removeAccountById(id: string) {
    const realIndex = accounts.value.findIndex(a => a.id === id);
    if (realIndex !== -1) {
      removeAccount(realIndex);
    }
  }

  async function loginMicrosoft() {
    loading.value = true;
    error.value = "";
    statusMessage.value = "Check the popup window to sign in...";
    try {
      const result = await invoke<Account>("login_microsoft");
      
      const existingIndex = accounts.value.findIndex(a => a.id === result.id);
      if (existingIndex !== -1) {
        accounts.value[existingIndex] = result;
        setActiveAccount(existingIndex);
      } else if (accounts.value.length < 3) {
        accounts.value.push(result);
        setActiveAccount(accounts.value.length - 1);
      } else {
        error.value = "Maximum 3 accounts allowed. Remove one first.";
        return;
      }
      
      await renderSkinHead();
      await saveSession();
    } catch (e: any) {
      error.value = e?.message || e?.toString() || "Microsoft login failed";
    } finally {
      loading.value = false;
      statusMessage.value = "Authenticating...";
    }
  }

  async function loginOffline(username: string) {
    loading.value = true;
    error.value = "";
    statusMessage.value = "Authenticating...";
    try {
      const result = await invoke<Account>("login_offline", { username });
      
      const existingIndex = accounts.value.findIndex(a => a.id === result.id);
      if (existingIndex !== -1) {
        accounts.value[existingIndex] = result;
        setActiveAccount(existingIndex);
      } else if (accounts.value.length < 3) {
        accounts.value.push(result);
        setActiveAccount(accounts.value.length - 1);
      } else {
        error.value = "Maximum 3 accounts allowed. Remove one first.";
        return;
      }
      
      renderedHeadUrl.value = null;
      await saveSession();
    } catch (e: any) {
      error.value = e?.message || e?.toString() || "Offline login failed";
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    accounts.value = [];
    activeAccountIndex.value = 0;
    renderedHeadUrl.value = null;
    sessionRestored.value = false;
    try {
      await invoke("clear_session");
    } catch (e) {
      console.error("Failed to clear session:", e);
    }
    try {
      localStorage.removeItem('sparkle_session');
    } catch (e) {
      console.error("Failed to clear localStorage:", e);
    }
  }

  async function removeAccount(index: number) {
    if (index < 0 || index >= accounts.value.length) return;
    accounts.value.splice(index, 1);
    if (activeAccountIndex.value >= accounts.value.length) {
      activeAccountIndex.value = Math.max(0, accounts.value.length - 1);
    }
    if (accounts.value.length === 0) {
      await logout();
    } else {
      await saveSession(); // Guardar después de eliminar
      renderSkinHead();
    }
  }

  async function saveSession() {
    try {
      await invoke("save_session", {
        accounts: accounts.value,
        active_index: activeAccountIndex.value
      });
    } catch (e) {
      console.error("Failed to save session via Tauri:", e);
    }
    try {
      localStorage.setItem('sparkle_session', JSON.stringify({
        accounts: accounts.value,
        activeIndex: activeAccountIndex.value
      }));
    } catch (e) {
      console.error("Failed to save session to localStorage:", e);
    }
  }

  async function loadSession(): Promise<boolean> {
    if (sessionRestored.value) return accounts.value.length > 0;
    
    let loaded = false;
    try {
      const result = await invoke<any | null>("load_session");
      if (result) {
        const data = typeof result === 'string' ? JSON.parse(result) : result;
        if (data.accounts && data.accounts.length > 0) {
          accounts.value = data.accounts;
          activeAccountIndex.value = data.activeIndex ?? 0;
          await renderSkinHead();
          loaded = true;
        }
      }
    } catch {
      // No session found in Tauri
    }
    
    if (!loaded) {
      try {
        const raw = localStorage.getItem('sparkle_session');
        if (raw) {
          const result = JSON.parse(raw);
          if (result.accounts && result.accounts.length > 0) {
            accounts.value = result.accounts;
            activeAccountIndex.value = result.activeIndex ?? 0;
            await renderSkinHead();
            loaded = true;
          }
        }
      } catch (e) {
        console.error("Failed to load session from localStorage:", e);
      }
    }
    
    sessionRestored.value = true;
    return loaded;
  }

  function clearError() {
    error.value = "";
  }

  return {
    accounts,
    account,
    activeAccountIndex,
    otherAccounts,
    loading,
    error,
    statusMessage,
    isAuthenticated,
    isPremium,
    username,
    accountType,
    avatarUrl,
    skinUrl,
    skinTextureKey,
    renderedHeadUrl,
    sessionRestored,
    renderSkinHead,
    setActiveAccount,
    switchAccount,
    removeAccountById,
    loginMicrosoft,
    loginOffline,
    logout,
    removeAccount,
    saveSession,
    loadSession,
    clearError,
  };
});