<script lang="ts">
  import { Bar, Line } from 'svelte-chartjs';
  import {
    Chart,
    Title,
    Tooltip,
    Legend,
    BarElement,
    LineElement,
    PointElement,
    CategoryScale,
    LinearScale,
    Colors,
    type ChartData,
    Filler
  } from 'chart.js';

  export let chartType: 'bar' | 'line' = 'bar';
  export let height = 350;
  export let borderWidth = 2;
  export let labels = ['Red', 'Blue', 'Yellow', 'Green', 'Purple', 'Orange'];
  export let datasetData = [12, 19, 3, 5, 2, 3];

  export let backgroundColors: string[] | undefined = [];
  let defaultBackgroundColors: string[] | undefined = [
    'rgba(255, 134, 159, 0.4)',
    'rgba(98, 182, 239, 0.4)',
    'rgba(255, 218, 128, 0.4)',
    'rgba(113, 205, 205, 0.4)',
    'rgba(170, 128, 252, 0.4)',
    'rgba(255, 177, 101, 0.4)',
    'rgba(201, 203, 207, 0.4)',
    'rgba(83, 162, 108, 0.4)',
    'rgba(253, 211, 92, 0.4)',
    'rgba(35, 107, 142, 0.4)',
    'rgba(185, 62, 77, 0.4)'
  ];
  export let borderColors: string[] | undefined = [];
  let defaultBorderColors: string[] | undefined = [
    'rgba(255, 134, 159, 1)',
    'rgba(98, 182, 239, 1)',
    'rgba(255, 218, 128, 1)',
    'rgba(113, 205, 205, 1)',
    'rgba(170, 128, 252, 1)',
    'rgba(255, 177, 101, 1)',
    'rgba(201, 203, 207, 1)',
    'rgba(83, 162, 108, 1)',
    'rgba(253, 211, 92, 1)',
    'rgba(35, 107, 142, 1)',
    'rgba(185, 62, 77, 1)'
  ];

  type RGBColor = {
    r: number;
    g: number;
    b: number;
  };

  function hexToRgb(hex: string): RGBColor {
    hex = hex.replace(/^#/, '');

    const bigint = parseInt(hex, 16);
    const r = (bigint >> 16) & 255;
    const g = (bigint >> 8) & 255;
    const b = bigint & 255;

    return { r, g, b };
  }

  function rgbToHex(r: number, g: number, b: number): string {
    return '#' + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1).toUpperCase();
  }

  function adjustColorBrightnessRgb(color: RGBColor, factor: number): RGBColor {
    return {
      r: Math.min(255, Math.max(0, Math.round(color.r * factor))),
      g: Math.min(255, Math.max(0, Math.round(color.g * factor))),
      b: Math.min(255, Math.max(0, Math.round(color.b * factor)))
    };
  }

  function adjustColorBrightness(color: string, factor: number): string {
    let rgbColor = adjustColorBrightnessRgb(hexToRgb(color), factor);

    return rgbToHex(rgbColor.r, rgbColor.g, rgbColor.b);
  }

  if (backgroundColors && backgroundColors.length > 0) {
    backgroundColors.forEach((bgColorHex) => {
      let borderColor = adjustColorBrightness(bgColorHex, 1.5);
      borderColors?.push(borderColor);
    });
  } else {
    backgroundColors = defaultBackgroundColors;
    borderColors = defaultBorderColors;
  }

  let data: ChartData<'bar' | 'line', (number | [number, number])[], unknown> = {
    labels: labels,
    datasets: [
      {
        label: 'Restaurant Score',
        borderWidth: borderWidth,
        data: datasetData,
        backgroundColor: backgroundColors,
        borderColor: borderColors,
        fill: chartType === 'line',
        pointStyle: 'circle',
        pointRadius: 8,
        pointHoverRadius: 10
      }
    ]
  };

  let delayed = true;

  let plugins;
  let linePlugins = {
    title: {
      // display: true
      // text: (ctx: { chart: { data: { datasets: { pointStyle: string }[] } } }) =>
      //   'Point Style: ' + ctx.chart.data.datasets[0].pointStyle
    },
    Filler
  };
  if (chartType === 'line') {
    plugins = linePlugins;
  }

  let options = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: plugins,
    animation: {
      onComplete: () => {
        delayed = true;
      },
      delay: (context: any) => {
        let delay = 0;
        if (context.type === 'data' && context.mode === 'default') {
          delay = context.dataIndex * 2000;
        }
        return delay;
      }
    },
    scales: {
      y: {
        min: 0,
        max: 10
      }
    }
  };

  if (chartType === 'line') {
    Chart.register(
      Title,
      Tooltip,
      Legend,
      LineElement,
      PointElement,
      CategoryScale,
      LinearScale,
      Colors,
      Filler
    );
  } else {
    Chart.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, Colors);
  }
</script>

{#if chartType === 'bar'}
  <Bar {data} {height} {options} />
{:else if chartType === 'line'}
  <Line {data} {height} {options} />
{/if}
