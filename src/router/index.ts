import { createRouter, createWebHashHistory } from "vue-router";
import { useAuthStore } from "../stores/auth";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "login",
      component: () => import("../views/LoginView.vue"),
      meta: { guest: true, title: "Login" },
    },
    {
      path: "/home",
      name: "home",
      component: () => import("../views/HomeView.vue"),
      meta: { requiresAuth: true, title: "Home" },
    },
    {
      path: "/:pathMatch(.*)*",
      redirect: "/",
    },
  ],
});

let authGuardInitialized = false;

router.beforeEach(async (to, _from, next) => {
  const auth = useAuthStore();
  
  // Update document title
  const title = to.meta.title as string;
  if (title) document.title = `${title} — Sparkle`;
  
  // Load session on first navigation if not already loaded
  if (!authGuardInitialized) {
    authGuardInitialized = true;
    if (!auth.sessionRestored) {
      const hasSession = await auth.loadSession();
      // If we have a session and we're going to login, redirect to home
      if (hasSession && to.meta.guest) {
        next("/home");
        return;
      }
    }
  }
  
  if (to.meta.requiresAuth && !auth.isAuthenticated) {
    next("/");
  } else if (to.meta.guest && auth.isAuthenticated) {
    next("/home");
  } else {
    next();
  }
});

export default router;
