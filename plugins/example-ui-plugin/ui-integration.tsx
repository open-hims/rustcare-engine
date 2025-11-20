/**
 * UI Integration Example for Example UI Plugin
 * 
 * This shows how to use the UI components registered by the WASM plugin
 * in a React/Remix application.
 */

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";

/**
 * Patient Dashboard Widget Component
 * Registered by the WASM plugin
 */
export function PatientDashboardWidget() {
  const [bmi, setBmi] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);

  const calculateBMI = async () => {
    setLoading(true);
    try {
      // Call the plugin function via API
      const response = await fetch('/api/v1/plugins/{plugin_id}/execute', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
        body: JSON.stringify({
          function_name: 'calculate_bmi',
          input_data: {
            weight_kg: 70.0,
            height_m: 1.75,
          },
        }),
      });

      const result = await response.json();
      if (result.success && result.data?.data?.bmi) {
        setBmi(result.data.data.bmi);
      }
    } catch (error) {
      console.error('Failed to calculate BMI:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Patient Dashboard Widget</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-gray-600 mb-4">
          This widget is provided by the Example UI Plugin (WASM)
        </p>
        <Button onClick={calculateBMI} disabled={loading}>
          {loading ? 'Calculating...' : 'Calculate BMI'}
        </Button>
        {bmi && (
          <div className="mt-4">
            <p className="text-lg font-semibold">BMI: {bmi.toFixed(2)}</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Health Metrics Chart Component
 * Registered by the WASM plugin
 */
export function HealthMetricsChart({ data }: { data: any }) {
  const [formatted, setFormatted] = useState<any>(null);

  useEffect(() => {
    const formatData = async () => {
      try {
        const response = await fetch('/api/v1/plugins/{plugin_id}/execute', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          credentials: 'include',
          body: JSON.stringify({
            function_name: 'format_health_data',
            input_data: data,
          }),
        });

        const result = await response.json();
        if (result.success && result.data?.data) {
          setFormatted(result.data.data);
        }
      } catch (error) {
        console.error('Failed to format health data:', error);
      }
    };

    if (data) {
      formatData();
    }
  }, [data]);

  if (!formatted) {
    return <div>Loading...</div>;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Health Metrics Chart</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {formatted.blood_pressure && (
            <div>
              <span className="font-medium">Blood Pressure: </span>
              <span>{formatted.blood_pressure}</span>
              {formatted.bp_category && (
                <span className="ml-2 text-sm text-gray-600">
                  ({formatted.bp_category})
                </span>
              )}
            </div>
          )}
          {formatted.heart_rate && (
            <div>
              <span className="font-medium">Heart Rate: </span>
              <span>{formatted.heart_rate}</span>
              {formatted.hr_status && (
                <span className="ml-2 text-sm text-gray-600">
                  ({formatted.hr_status})
                </span>
              )}
            </div>
          )}
          {formatted.temperature && (
            <div>
              <span className="font-medium">Temperature: </span>
              <span>{formatted.temperature}</span>
              {formatted.temp_status && (
                <span className="ml-2 text-sm text-gray-600">
                  ({formatted.temp_status})
                </span>
              )}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Quick Action Button Component
 * Registered by the WASM plugin
 */
export function QuickActionButton({ 
  action, 
  onAction 
}: { 
  action: string; 
  onAction: () => void;
}) {
  return (
    <Button 
      onClick={onAction}
      className="w-full"
      variant="outline"
    >
      <span className="mr-2">⚡</span>
      {action}
    </Button>
  );
}

/**
 * Example usage in a page component
 */
export default function PluginExamplePage() {
  const healthData = {
    systolic: 120,
    diastolic: 80,
    heart_rate: 72,
    temperature: 36.5,
  };

  return (
    <div className="space-y-6 p-6">
      <h1 className="text-3xl font-bold">Plugin UI Components Example</h1>
      
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <PatientDashboardWidget />
        <HealthMetricsChart data={healthData} />
      </div>

      <div className="grid grid-cols-3 gap-4">
        <QuickActionButton 
          action="Quick View" 
          onAction={() => console.log('Quick view clicked')} 
        />
        <QuickActionButton 
          action="Export Data" 
          onAction={() => console.log('Export clicked')} 
        />
        <QuickActionButton 
          action="Share Report" 
          onAction={() => console.log('Share clicked')} 
        />
      </div>
    </div>
  );
}

