export interface ResumeItem {
  id: number
  title: string
  content: string
  created_at: number
  updated_at: number
}

export interface EducationItem {
  school: string
  major: string
  degree: string
  startDate: string
  endDate: string
}

export interface WorkItem {
  company: string
  position: string
  startDate: string
  endDate: string
  description: string
}

export interface ProjectItem {
  name: string
  role: string
  startDate: string
  endDate: string
  description: string
  technologies: string
}

export interface PersonalInfo {
  name: string
  gender: string
  birthDate: string
  phone: string
  email: string
  location: string
  education: EducationItem[]
  workExperience: WorkItem[]
  projects: ProjectItem[]
  skills: string
  selfEvaluation: string
  certificates: string
}

export interface ResumeTemplate {
  id: string
  name: string
  description: string
  icon: string
}

export interface QuoteItem {
  id: number
  content: string
  source: string | null
  type: string
}

export interface NewsSourceItem {
  id: number
  name: string
  url: string
  category: string
  feed_type: string
}