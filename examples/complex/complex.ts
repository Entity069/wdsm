interface UserProfile {
    id: number;
    username: string;
    isActive: boolean;
    tags?: string[];
}

export function processUsers(users: UserProfile[], requireActive: boolean): Promise<string> {
    const totalUsers = users.length;
    const activeUsers = users.filter(u => u.isActive);
    
    if (requireActive && activeUsers.length === 0) {
        return Promise.resolve("Error: No active users found");
    }

    const processedUsers = users.map(u => {
        const tagCount = u.tags ? u.tags.length : 0;
        return `${u.username} (ID: ${u.id}, Tags: ${tagCount})`;
    });

    return Promise.resolve(`Successfully processed ${totalUsers} users. Active: ${activeUsers.length}. Details: ${processedUsers.join(", ")}`);
}
