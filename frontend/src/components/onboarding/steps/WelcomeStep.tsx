import React from 'react';
import { Download, Mic, Eye } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';

export function WelcomeStep() {
  const { goNext } = useOnboarding();

  const steps = [
    {
      icon: Download,
      title: 'Download the speech model',
      detail: 'One time, about 670 MB. It runs on your Mac, so your calls stay private.',
    },
    {
      icon: Mic,
      title: 'Allow the microphone and audio',
      detail: 'macOS asks you. Click Allow both times so Graphite can hear you and the people on the call.',
    },
    {
      icon: Eye,
      title: 'Optional: names and screen snapshots',
      detail: 'For Google Meet only. Two switches in System Settings, and you can skip them.',
    },
  ];

  return (
    <OnboardingContainer
      title="Welcome to Graphite"
      description="Listens to your calls and writes the transcript on this Mac. Audio isn't kept."
      step={1}
      hideProgress={true}
    >
      <div className="flex flex-col items-center space-y-8">
        <div className="w-full max-w-md bg-white rounded-lg border border-gray-200 shadow-sm p-6 space-y-5">
          <p className="text-sm font-medium text-gray-900">Setup takes three quick steps:</p>
          {steps.map((step, index) => {
            const Icon = step.icon;
            return (
              <div key={index} className="flex items-start gap-3">
                <div className="flex-shrink-0 mt-0.5 w-7 h-7 rounded-full bg-gray-100 flex items-center justify-center">
                  <Icon className="w-4 h-4 text-gray-700" />
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-900">
                    {index + 1}. {step.title}
                  </p>
                  <p className="text-sm text-gray-600 leading-relaxed">{step.detail}</p>
                </div>
              </div>
            );
          })}
        </div>

        <div className="w-full max-w-xs space-y-3">
          <Button
            onClick={goNext}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white"
          >
            Start setup
          </Button>
          <p className="text-xs text-center text-gray-500">About 3 minutes, mostly the download</p>
        </div>
      </div>
    </OnboardingContainer>
  );
}
