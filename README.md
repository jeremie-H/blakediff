# blakediff

_⚠ This repository is only for testing purpose. It shouldn't be used on production._

A simple first version to make hash on all files in a directory and generate a report file with this format :

```shell
<hash_1> <path_file_1>
<hash_2> <path_file_2>
```
## Command `generate`
Use this first subcommand to generate a report file with all hashes and path's files  
```shell
blakediff generate <path_directory>  > report_file_1
```
the option `--parallel` or `-p` can be used to walk directories tree in multithreading.  


_Exemples :_
```
blakediff generate -p ~/Music > ~/musiques_hash_local.txt
blakediff generate -p /mnt/smbmount/Music > ~/musiques_hash_smb.txt
```

## Command `compare`

```shell
blakediff compare <report_file_1> <report_file_2>
```

Generate all missing hash from the path1 in path2

_Exemples :_
```
blakediff compare ~/musiques_hash_local.txt ~/musiques_hash_smb.txt
```

_Results exemples:_
```
not found in ~/musiques_hash_smb.txt : /home/jeremie/Music/my_song234.mp3
not found in ~/musiques_hash_smb.txt : /home/jeremie/Music/my_song456.mp3
```