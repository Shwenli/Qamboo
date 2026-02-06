import os

def generate_config(group_id, num_parties=3):
    base_port = 9000 + group_id * 1000
    directory = "local/"+ str(group_id)
    
    if not os.path.exists(directory):
        os.makedirs(directory)
    
    parties_config = ""
    for i in range(num_parties):
        port = base_port + i
        parties_config += "[[parties]]\n"
        parties_config += f"id = {i}\n"
        parties_config += f"dns_name = \"127.0.0.1:{port}\"\n"

    for my_id in range(num_parties):
        my_port = base_port + my_id
        content = f"my_id = {my_id}\n"
        content += f"bind_addr = \"0.0.0.0:{my_port}\"\n"
        content += parties_config
        
        filename = os.path.join(directory, f"config_party{my_id}.toml")
        with open(filename, "w") as f:
            f.write(content)

if __name__ == "__main__":
    # Generate for groups 0 to 9
    for i in range(10):
        generate_config(i)
