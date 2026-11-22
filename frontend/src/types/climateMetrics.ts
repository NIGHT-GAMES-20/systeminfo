export interface Coordinates {
  lat: number;
  lng: number;
}

export interface CurrentWeatherUnits {
  temperature_2m: string;
  apparent_temperature: string;
  relative_humidity_2m: string;
  surface_pressure: string;
  wind_speed_10m: string;
  wind_direction_10m: string;
  weather_code: string;
}

export interface CurrentWeatherData {
  time: string;
  interval: number;
  temperature_2m: number;
  apparent_temperature: number;
  relative_humidity_2m: number;
  surface_pressure: number;
  wind_speed_10m: number;
  wind_direction_10m: number;
  weather_code: number;
}

export interface OpenMeteoResponse {
  latitude: number;
  longitude: number;
  generationtime_ms: number;
  utc_offset_seconds: number;
  timezone: string;
  timezone_abbreviation: string;
  elevation: number;
  current_units: CurrentWeatherUnits;
  current: CurrentWeatherData;
}

export interface NominatimResult {
  lat: string;
  lon: string;
  display_name: string[] | string;
}

export type CompassDirection =
  | 'N' | 'NNE' | 'NE' | 'ENE'
  | 'E' | 'ESE' | 'SE' | 'SSE'
  | 'S' | 'SSW' | 'SW' | 'WSW'
  | 'W' | 'WNW' | 'NW' | 'NNW';
