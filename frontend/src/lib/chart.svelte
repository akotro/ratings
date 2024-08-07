<script lang="ts">
  import { Bar } from 'svelte-chartjs';
  import {
    Chart,
    Title,
    Tooltip,
    Legend,
    BarElement,
    CategoryScale,
    LinearScale,
    Colors,
    type ChartData
  } from 'chart.js';

  export let height = 350;
  export let borderWidth = 2;
  export let backgroundColor = [
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
  export let borderColor = [
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
  export let labels = ['Red', 'Blue', 'Yellow', 'Green', 'Purple', 'Orange'];
  export let datasetData = [12, 19, 3, 5, 2, 3];

  let data: ChartData<'bar', (number | [number, number])[], unknown> = {
    labels: labels,
    datasets: [
      {
        label: 'Restaurant Score',
        borderWidth: borderWidth,
        data: datasetData,
        backgroundColor: backgroundColor,
        borderColor: borderColor
      }
    ]
  };

  let options = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      y: {
        min: 0,
        max: 10
      }
    }
  };

  Chart.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, Colors);
</script>

<Bar {data} {height} {options} />
