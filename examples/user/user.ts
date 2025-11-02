type User = {
    id: number;
    name: string;
    username: string;
    age: number;
    isActive: boolean;
    roles: string[];
}

export function createUser(
    name: string,
    username: string, age: number): User {
    const user: User = {
        id: Math.floor(Math.random() * 100000),
        name,
        username,
        age,
        isActive: true,
        roles: ["user"],
    };
    
    return user;
}