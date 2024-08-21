export interface User {
  id: string;
  username: string;
  password: string;
  color: string;
  token: string;
  groupMembership: GroupMembership | null;
}

export enum Role {
  Admin = 'Admin',
  Member = 'Member'
}

export interface Group {
  id: string;
  name: string;
  description: string | null;
  created_at: Date;
  updated_at: Date;
}

export interface GroupMembership {
  id: number;
  group_id: string;
  group: Group;
  user_id: string;
  role: Role;
  created_at: Date;
  updated_at: Date;
}

export interface Restaurant {
  id: string;
  cuisine: string;
}

export enum Period {
  Q1 = 0,
  Q2 = 1,
  Q3 = 2,
  Q4 = 3
}

export interface Rating {
  id: number;
  group_id: string;
  restaurant_id: string;
  user_id: string;
  username: string;
  score: number;
  created_at: Date;
  updated_at: Date;
  period: Period;
  color: string | undefined;
}

export interface NewRating {
  restaurant_id: string;
  user_id: string;
  username: string;
  score: number;
  group_id: string;
}

export interface RatingsByPeriod {
  current_year: number;
  current_period: Period;
  current_period_ratings: Array<Rating>;
  historical_ratings: Array<AverageRatingPerPeriod>;
}

export interface AverageRatingPerPeriod {
  year: number;
  period: Period;
  average_score: number;
}
