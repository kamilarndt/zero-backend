/**
 * useWidgetData - Custom hook for managing widget data via SSE
 * Handles widget state updates from the backend
 */

import { useState, useCallback, useEffect } from 'react';
import { sseBus } from '../sseBus';

export interface WidgetData {
  [key: string]: unknown;
}

interface UseWidgetDataOptions {
  widgetId: string;
  initialData?: WidgetData;
}

interface UseWidgetDataReturn {
  data: WidgetData;
  updateData: (updates: Partial<WidgetData>) => void;
  setData: (data: WidgetData) => void;
  isLoading: boolean;
  error: string | null;
}

export function useWidgetData(
  options: UseWidgetDataOptions
): UseWidgetDataReturn {
  const { widgetId, initialData = {} } = options;

  const [data, setData] = useState<WidgetData>(initialData);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Partial update helper
  const updateData = useCallback((updates: Partial<WidgetData>) => {
    setData((prev) => ({ ...prev, ...updates }));
  }, []);

  // SSE integration - listen for widget data updates
  useEffect(() => {
    const unsubscribe = sseBus.subscribeWidget(widgetId, (event) => {
      if (event.type === 'data_update') {
        updateData(event.data as WidgetData);
      } else if (event.type === 'loading_start') {
        setIsLoading(true);
        setError(null);
      } else if (event.type === 'loading_end') {
        setIsLoading(false);
      } else if (event.type === 'error') {
        setIsLoading(false);
        setError(event.data as string);
      }
    });

    return unsubscribe;
  }, [widgetId, updateData]);

  return {
    data,
    updateData,
    setData,
    isLoading,
    error,
  };
}

/**
 * useCareerConciergeData - Specific hook for Career Concierge widget
 */
export interface CareerTask {
  id: string;
  text: string;
  completed: boolean;
  priority: 'high' | 'medium' | 'low';
}

export interface CareerConciergeData {
  tasks: CareerTask[];
  interviewProgress: {
    current: number;
    total: number;
  };
  stats: {
    tasksCompleted: number;
    skillsIdentified: number;
    patternsDetected: number;
  };
  currentAgent: string;
}

export function useCareerConciergeData() {
  const { data, updateData, isLoading } = useWidgetData({
    widgetId: 'career-concierge',
    initialData: {
      tasks: [],
      interviewProgress: { current: 0, total: 10 },
      stats: {
        tasksCompleted: 0,
        skillsIdentified: 0,
        patternsDetected: 0,
      },
      currentAgent: 'career-concierge',
    },
  });

  const toggleTask = useCallback((taskId: string) => {
    const tasks = (data.tasks as CareerTask[]) || [];
    updateData({
      tasks: tasks.map((task) =>
        task.id === taskId ? { ...task, completed: !task.completed } : task
      ),
    });
  }, [data, updateData]);

  const addTask = useCallback((text: string, priority: CareerTask['priority'] = 'medium') => {
    const tasks = (data.tasks as CareerTask[]) || [];
    const newTask: CareerTask = {
      id: `task-${Date.now()}`,
      text,
      completed: false,
      priority,
    };
    updateData({
      tasks: [...tasks, newTask],
    });
  }, [data.tasks, updateData]);

  return {
    data: data as unknown as CareerConciergeData,
    updateData,
    isLoading,
    toggleTask,
    addTask,
  };
}

/**
 * useGraphicDesignerData - Specific hook for Graphic Designer widget
 */
export interface ColorSwatch {
  hex: string;
  name: string;
}

export interface DesignDNA {
  colors: ColorSwatch[];
  typography: {
    heading: string;
    body: string;
  };
  spacing: {
    base: number;
    scale: number;
  };
}

export interface CVStatus {
  format: 'pdf' | 'word' | 'latex';
  ready: boolean;
  url?: string;
}

export interface GraphicDesignerData {
  designDNA: DesignDNA | null;
  cvs: CVStatus[];
  portfolioPreview: string | null;
  isGenerating: boolean;
}

export function useGraphicDesignerData() {
  const { data, updateData, isLoading } = useWidgetData({
    widgetId: 'graphic-designer',
    initialData: {
      designDNA: null,
      cvs: [],
      portfolioPreview: null,
      isGenerating: false,
    },
  });

  const updateDesignDNA = useCallback((designDNA: DesignDNA) => {
    updateData({ designDNA });
  }, [updateData]);

  const setCVStatus = useCallback((format: CVStatus['format'], ready: boolean, url?: string) => {
    const cvs = [...((data.cvs as CVStatus[]) || [])];
    const existing = cvs.findIndex((cv) => cv.format === format);

    if (existing >= 0) {
      cvs[existing] = { format, ready, url };
    } else {
      cvs.push({ format, ready, url });
    }

    updateData({ cvs });
  }, [data.cvs, updateData]);

  return {
    data: data as unknown as GraphicDesignerData,
    updateData,
    isLoading,
    updateDesignDNA,
    setCVStatus,
  };
}
