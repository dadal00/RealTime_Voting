<script lang="ts">
    import { cn } from "./utils";
    import { onMount } from "svelte";

    export let number = 10;
    let meteorStyles: any = [];
    let off_screen: number;
    let changeMeteors = (num: number) => {
      meteorStyles = [];
      const styles = [...new Array(num)].map(() => ({
        top: -20,
        left: Math.floor(Math.random() * (off_screen + window.innerWidth) - off_screen) + "px",
        animationDelay: Math.random() * 2 + 0.2 + "s",
        animationDuration: Math.floor(Math.random() * 15 + 2.9) + "s",
      }));
      meteorStyles = styles;
    };
    onMount(() => {
      off_screen = Math.tan(55 * (Math.PI / 180)) * window.innerHeight
      changeMeteors(number)
    });
  </script>
  
  {#each meteorStyles as style, idx}
    <span
      id="meteor-{idx+1}"
      class={cn(
        "pointer-events-none absolute left-1/2 top-1/2 size-[2.4px] rotate-[215deg] animate-meteor rounded-full bg-slate-500 shadow-[0_0_0_1px_#ffffff10] z-[-1]"
      )}
      style="top: {style.top}px; left: {style.left}; animation-delay: {style.animationDelay}; animation-duration: {style.animationDuration};"
    >
      <div
        class="pointer-events-none absolute top-1/2 -z-10 h-px w-[50px] -translate-y-1/2 bg-gradient-to-r from-slate-500 via-blue-600/30 to-transparent z-[-1]"
      ></div>
    </span>
  {/each}
  