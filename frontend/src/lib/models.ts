export interface User {
	id: string;
	username: string;
	password: string;
	token: string;
}

export interface Restaurant {
	id: string;
	cuisine: string;
}

export interface Rating {
	id: number;
	restaurant_id: string;
	user_id: string;
	username: string;
	score: number;
}
