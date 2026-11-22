import { ref, computed } from 'vue';
import axios from 'axios';
import type { NominatimResult, OpenMeteoResponse, CompassDirection, CurrentWeatherData, CurrentWeatherUnits  } from '../types/climateMetrics'

const address = ref<string>("");
const nominationRes = ref<NominatimResult | null>(null);
const rawMetrics = ref<OpenMeteoResponse | null>(null);
const isLoading = ref<boolean>(false);

export function useClimateMetrics() {

  async function updateClimateMetrics() {
    isLoading.value = true;
    if (!nominationRes.value) await getCoordinates();
    const url = "https://api.open-meteo.com/v1/forecast";
    try {
      const response = await axios.get<OpenMeteoResponse>(url, {
        params: {
          latitude: nominationRes.value?.lat,
          longitude: nominationRes.value?.lon,
          current: [
            "temperature_2m",
            "apparent_temperature",
            "relative_humidity_2m",
            "surface_pressure",
            "wind_speed_10m",
            "wind_direction_10m",
            "weather_code"
          ].join(",")
        }
      });

      rawMetrics.value = response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        console.error("API Error:", error?.response?.data || error.message);
      } else {
        console.error("Unexpected Error:", error);
      }
    } finally {
      isLoading.value = false;
    }
  }

  async function getCoordinates() {
    if (!address.value) address.value = localStorage.getItem("targetAddress") ?? "London"

    const url = `https://nominatim.openstreetmap.org/search?q=${encodeURIComponent(address.value)}&format=json&limit=1`;
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
      const data: NominatimResult[] = await response.json();

      if (data && data.length > 0) {
        nominationRes.value = {
          lat: data[0].lat,
          lon: data[0].lon,
          display_name: typeof data[0].display_name === 'string' ? data[0].display_name.split(',') : data[0].display_name
        };
      }
      return null;
    } catch (error) {
      console.error("Error fetching coordinates:", error);
      return null;
    }
  }

  async function updateLocation(addr: string) {
    if (!addr) return;
    localStorage.setItem("targetAddress", addr);
    address.value = addr;
    await getCoordinates();
  }

  function getSkyCondition(code: number | undefined): string {
    const wmoCodes: Record<number, string> = {
      0: "Clear sky",
      1: "Mainly clear", 2: "Partly cloudy", 3: "Overcast",
      45: "Fog", 48: "Depositing rime fog",
      51: "Light drizzle", 53: "Moderate drizzle", 55: "Dense drizzle",
      61: "Slight rain", 63: "Moderate rain", 65: "Heavy rain",
      71: "Slight snow fall", 73: "Moderate snow fall", 75: "Heavy snow fall",
      77: "Snow grains",
      80: "Slight rain showers", 81: "Moderate rain showers", 82: "Violent rain showers",
      85: "Slight snow showers", 86: "Heavy snow showers",
      95: "Thunderstorm", 96: "Thunderstorm with slight hail", 99: "Thunderstorm with heavy hail",
      100: "Failed To Fetch"
    };

    if (code === undefined) return wmoCodes[100];
    return wmoCodes[code] ?? "Unknown";
  }


  function getWindDirection(degrees: number | undefined): CompassDirection {
    const normalizedDegrees = ((degrees ?? 0  % 360) + 360) % 360;

    const index = Math.floor((normalizedDegrees + 11.25) / 22.5);

    const directions: CompassDirection[] = [
      'N', 'NNE', 'NE', 'ENE',
      'E', 'ESE', 'SE', 'SSE',
      'S', 'SSW', 'SW', 'WSW',
      'W', 'WNW', 'NW', 'NNW'
    ];
    return directions[index % 16];
  }

  const climateData = computed((): { current?: CurrentWeatherData; units?: CurrentWeatherUnits, resolvedAddress?: string[] | string } => {
    return {
      current: rawMetrics.value?.current,
      units: rawMetrics.value?.current_units,
      resolvedAddress: nominationRes.value?.display_name,
    }
  })

  return {
    isLoading,
    updateClimateMetrics,
    getCoordinates,
    updateLocation,
    getSkyCondition,
    getWindDirection,
    climateData
  }
}
