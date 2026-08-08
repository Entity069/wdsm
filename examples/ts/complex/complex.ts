interface GeoLocation {
    lat: number;
    lng: number;
}

interface Address {
    street: string;
    city: string;
    geo: GeoLocation;
}

interface UserProfile {
    id: number;
    username: string;
    isActive: boolean;
    address: Address;
    tags?: string[];
}

interface ProcessSummary {
    totalUsers: number;
    activeUsers: number;
    primaryCity: string;
    details: string[];
}

export function processUsers(users: UserProfile[], requireActive: boolean): ProcessSummary {
    const totalUsers = users.length;
    const activeUsers = users.filter(u => u.isActive);

    const primaryCity = users.length > 0 ? users[0].address.city : "Unknown";

    const details = users.map(u => {
        const tagCount = u.tags ? u.tags.length : 0;
        return `${u.username} (ID: ${u.id}, City: ${u.address.city}, Lat: ${u.address.geo.lat}, Lng: ${u.address.geo.lng}, Tags: ${tagCount})`;
    });

    return {
        totalUsers,
        activeUsers: activeUsers.length,
        primaryCity,
        details
    };
}
