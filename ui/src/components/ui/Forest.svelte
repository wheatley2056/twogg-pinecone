<script lang="ts">
  // Rein dekorativ, ohne Eigenschaften.
</script>

<!-- Wald-Illustration der Spielen-Bühne (Design-System Abschnitt 10): gestufte Tannen, Mond,
     Hügel und Zapfenschuppen auf der rechten Hälfte. Farben kommen aus dem Thema. -->
<svg class="forest" viewBox="0 0 1280 658" preserveAspectRatio="xMaxYMid slice" aria-hidden="true">
  <defs>
    <path id="pc-tree" d="M0 0 L28 45 L16 45 L44 90 L26 90 L56 140 L-56 140 L-26 90 L-44 90 L-16 45 L-28 45 Z M-6 140h12v14h-12z" />
    <pattern id="pc-scales" width="28" height="20" patternUnits="userSpaceOnUse">
      <path class="scale" d="M0 10 Q7 0 14 10 Q21 0 28 10 M-7 20 Q0 10 7 20 Q14 10 21 20 Q28 10 35 20" fill="none" stroke-width="1.2" />
    </pattern>
  </defs>
  <rect class="scales-area" x="560" y="0" width="720" height="658" fill="url(#pc-scales)" />
  <circle cx="1000" cy="140" r="48" class="moon" />
  <circle cx="1018" cy="128" r="48" class="moon-shade" />
  <path d="M560 470 Q760 400 930 440 T1280 410 V658 H560Z" class="hill" />
  <use href="#pc-tree" transform="translate(770 300) scale(1.15)" class="back" />
  <use href="#pc-tree" transform="translate(930 250) scale(1.45)" class="back" />
  <use href="#pc-tree" transform="translate(1100 290) scale(1.2)" class="back" />
  <use href="#pc-tree" transform="translate(1240 240) scale(1.5)" class="back" />
  <use href="#pc-tree" transform="translate(680 270) scale(1.6)" class="front" />
  <use href="#pc-tree" transform="translate(840 200) scale(2)" class="front" />
  <use href="#pc-tree" transform="translate(1180 160) scale(2.3)" class="front" />
  <rect x="560" y="505" width="720" height="153" class="front" />
</svg>

<style>
  .forest {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .scale {
    stroke: var(--accent);
    stroke-opacity: 0.1;
  }

  :global(:root[data-pattern="off"]) .scales-area {
    display: none;
  }

  .moon {
    fill: var(--forest-moon);
  }

  .moon-shade {
    fill: var(--stage);
    opacity: 0.3;
  }

  .hill {
    fill: var(--forest-hill);
  }

  .back {
    fill: var(--forest-back);
  }

  .front {
    fill: var(--forest-front);
  }
</style>
